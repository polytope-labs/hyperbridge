use std::{
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	sync::Mutex,
	time::{Duration, Instant},
};

use axum::{
	extract::{ConnectInfo, Request, State},
	http::{HeaderMap, Method, StatusCode},
	middleware::Next,
	response::{IntoResponse, Response},
	Json,
};
use serde_json::json;

use crate::routes::AppState;

const WINDOW: Duration = Duration::from_secs(60);
const LOGINS_PER_WINDOW: u32 = 20;
const WRITES_PER_WINDOW: u32 = 120;

/// Counts each visitor's requests in fixed one minute windows, separately for logins and other
/// writes.
#[derive(Default)]
pub struct RateLimiter {
	windows: Mutex<HashMap<(IpAddr, bool), (Instant, u32)>>,
}

impl RateLimiter {
	fn allow(&self, visitor: IpAddr, login: bool) -> bool {
		let limit = if login { LOGINS_PER_WINDOW } else { WRITES_PER_WINDOW };
		let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		let now = Instant::now();
		let (started, count) = windows.entry((visitor, login)).or_insert((now, 0));
		if now.duration_since(*started) >= WINDOW {
			(*started, *count) = (now, 0);
		}
		*count += 1;
		*count <= limit
	}

	/// Forgets windows that have ended, so the map does not grow with every visitor ever seen.
	pub fn prune(&self) {
		let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		windows.retain(|_, (started, _)| started.elapsed() < WINDOW);
	}
}

/// The visitor's address. Requests through the frontend's `/api` proxy arrive from the frontend, so
/// the last `X-Forwarded-For` entry, the one our proxy appended, is used. Earlier entries come from
/// the client and could be made up.
fn visitor(headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
	headers
		.get_all("x-forwarded-for")
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(','))
		.last()
		.and_then(|address| address.trim().parse().ok())
		.unwrap_or(peer.ip())
}

/// Turns away visitors sending too many writes or login attempts. Reads are not limited.
pub async fn limit(
	State(state): State<AppState>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Response {
	if matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
		return next.run(request).await;
	}
	let login = request.uri().path().starts_with("/auth/");
	if !state.limiter.allow(visitor(request.headers(), peer), login) {
		let message = json!({ "message": "Too many requests, please try again in a minute" });
		return (StatusCode::TOO_MANY_REQUESTS, Json(message)).into_response();
	}
	next.run(request).await
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn limits_each_visitor_and_bucket_separately() {
		let limiter = RateLimiter::default();
		let (alice, bob) = ("10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap());
		assert!((0..LOGINS_PER_WINDOW).all(|_| limiter.allow(alice, true)));
		assert!(!limiter.allow(alice, true));
		assert!(limiter.allow(alice, false), "writes have their own allowance");
		assert!(limiter.allow(bob, true), "another visitor is unaffected");
	}

	#[test]
	fn takes_the_address_our_proxy_appended() {
		let peer = "192.168.1.1:5000".parse().unwrap();
		let mut headers = HeaderMap::new();
		assert_eq!(visitor(&headers, peer), peer.ip());
		headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
		assert_eq!(visitor(&headers, peer), "5.6.7.8".parse::<IpAddr>().unwrap());
	}
}
