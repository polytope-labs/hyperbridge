use std::{
	collections::{BTreeMap, HashMap},
	net::{IpAddr, Ipv6Addr, SocketAddr},
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};

use anyhow::Context;
use axum::{
	extract::{ConnectInfo, Request, State},
	http::{HeaderMap, Method, StatusCode},
	middleware::Next,
	response::{IntoResponse, Response},
	Json,
};
use ipnet::IpNet;
use serde_json::json;

use crate::routes::AppState;

pub const WINDOW: Duration = Duration::from_secs(60);
const LOGINS_PER_WINDOW: u32 = 20;
const WRITES_PER_WINDOW: u32 = 120;
/// Visitors tracked at once, about 50MB of windows. Once every slot holds a live window, visitors
/// without one are let through untracked until windows end, so a flood of addresses can neither
/// grow the map without bound nor turn everyone else away.
const MAX_VISITORS: usize = 1_000_000;
/// Scan height streams one visitor may hold open, so no one visitor takes every stream.
const STREAMS_PER_VISITOR: usize = 8;

/// Counts each visitor's requests in fixed one minute windows, separately for logins and other
/// writes, and the scan height streams each visitor holds open.
pub struct RateLimiter {
	windows: Mutex<HashMap<(IpAddr, bool), (Instant, u32)>>,
	streams: Mutex<BTreeMap<IpAddr, usize>>,
	/// Proxies whose `X-Forwarded-For` entries are believed
	trusted: Vec<IpNet>,
	capacity: usize,
}

/// One open stream counted against its visitor, given back when the stream ends.
pub struct StreamSlot {
	limiter: Arc<RateLimiter>,
	visitor: IpAddr,
}

impl Drop for StreamSlot {
	fn drop(&mut self) {
		let mut streams =
			self.limiter.streams.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		if let Some(open) = streams.get_mut(&self.visitor) {
			*open -= 1;
			if *open == 0 {
				streams.remove(&self.visitor);
			}
		}
	}
}

impl RateLimiter {
	pub fn new(trusted: Vec<IpNet>) -> Self {
		Self {
			windows: Mutex::default(),
			streams: Mutex::default(),
			trusted,
			capacity: MAX_VISITORS,
		}
	}

	/// Counts a stream against `visitor`, unless it already holds as many as it may.
	pub fn open_stream(self: &Arc<Self>, visitor: IpAddr) -> Option<StreamSlot> {
		let visitor = bucket(visitor);
		let mut streams = self.streams.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		let open = streams.entry(visitor).or_default();
		if *open >= STREAMS_PER_VISITOR {
			return None;
		}
		*open += 1;
		Some(StreamSlot { limiter: self.clone(), visitor })
	}

	/// The proxies in `TRUSTED_PROXIES`, a comma separated list of addresses and CIDR ranges.
	pub fn trusted_proxies(list: &str) -> anyhow::Result<Vec<IpNet>> {
		list.split(',')
			.map(str::trim)
			.filter(|entry| !entry.is_empty())
			.map(|entry| {
				entry
					.parse::<IpNet>()
					.or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
					.with_context(|| {
						format!("{entry} in TRUSTED_PROXIES is not an address or range")
					})
			})
			.collect()
	}

	fn allow(&self, visitor: IpAddr, login: bool) -> bool {
		let limit = if login { LOGINS_PER_WINDOW } else { WRITES_PER_WINDOW };
		let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		let key = (bucket(visitor), login);
		if !windows.contains_key(&key) && windows.len() >= self.capacity {
			return true;
		}
		let now = Instant::now();
		let (started, count) = windows.entry(key).or_insert((now, 0));
		if now.duration_since(*started) >= WINDOW {
			(*started, *count) = (now, 0);
		}
		*count += 1;
		*count <= limit
	}

	/// Forgets windows that have ended, so the map only holds visitors seen in the last minute.
	pub fn prune(&self) {
		let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		windows.retain(|_, (started, _)| started.elapsed() < WINDOW);
		if windows.len() >= self.capacity {
			tracing::warn!(
				target: crate::LOG_TARGET,
				"the rate limiter is tracking {} visitors, so new ones go unlimited",
				windows.len()
			);
		}
	}

	fn is_trusted(&self, address: IpAddr) -> bool {
		self.trusted.iter().any(|proxy| proxy.contains(&address))
	}

	/// The visitor's address. `X-Forwarded-For` is only read when the request came from a trusted
	/// proxy, and then from the right, skipping the trusted proxies that appended to it. The first
	/// other entry is the address the outermost trusted proxy saw, so anything a client wrote to
	/// the left of it is ignored.
	pub(crate) fn visitor(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
		let peer = peer.ip().to_canonical();
		if !self.is_trusted(peer) {
			return peer;
		}
		let hops = headers
			.get_all("x-forwarded-for")
			.iter()
			.filter_map(|value| value.to_str().ok())
			.flat_map(|value| value.split(','))
			.collect::<Vec<_>>();
		hops.into_iter()
			.rev()
			.map(|hop| hop.trim().parse::<IpAddr>().ok().map(|hop| hop.to_canonical()))
			.try_fold(peer, |last, hop| match hop {
				Some(hop) if self.is_trusted(hop) => Ok(hop),
				Some(hop) => Err(hop),
				// An entry that is not an address cannot be followed, so the last trusted hop is
				// used
				None => Err(last),
			})
			.unwrap_or_else(|visitor| visitor)
	}
}

/// IPv6 visitors usually hold a whole /64, so they are counted by it.
fn bucket(visitor: IpAddr) -> IpAddr {
	match visitor.to_canonical() {
		IpAddr::V6(address) =>
			IpAddr::V6(Ipv6Addr::from(u128::from(address) & !(u64::MAX as u128))),
		address => address,
	}
}

/// Turns away visitors sending too many writes or login attempts. Reads are not limited, except
/// login challenges, which are created by a GET.
pub async fn limit(
	State(state): State<AppState>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Response {
	let path = request.uri().path();
	let read = matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS);
	if read && !path.starts_with("/auth/login/") {
		return next.run(request).await;
	}
	let login = path.starts_with("/auth/");
	let visitor = state.limiter.visitor(request.headers(), peer);
	if !state.limiter.allow(visitor, login) {
		let message = json!({ "message": "Too many requests, please try again in a minute" });
		return (StatusCode::TOO_MANY_REQUESTS, Json(message)).into_response();
	}
	next.run(request).await
}

#[cfg(test)]
mod tests {
	use super::*;

	fn ip(address: &str) -> IpAddr {
		address.parse().unwrap()
	}

	#[test]
	fn limits_each_visitor_and_bucket_separately() {
		let limiter = RateLimiter::new(vec![]);
		let (alice, bob) = (ip("10.0.0.1"), ip("10.0.0.2"));
		assert!((0..LOGINS_PER_WINDOW).all(|_| limiter.allow(alice, true)));
		assert!(!limiter.allow(alice, true));
		assert!(limiter.allow(alice, false), "writes have their own allowance");
		assert!(limiter.allow(bob, true), "another visitor is unaffected");
	}

	#[test]
	fn counts_an_ipv6_visitor_by_its_64() {
		let limiter = RateLimiter::new(vec![]);
		assert!((0..LOGINS_PER_WINDOW)
			.all(|index| limiter.allow(ip(&format!("2001:db8:1:2::{index:x}")), true)));
		assert!(!limiter.allow(ip("2001:db8:1:2:ffff::1"), true), "same /64");
		assert!(limiter.allow(ip("2001:db8:1:3::1"), true), "another /64");
	}

	#[test]
	fn holds_a_bounded_number_of_visitors() {
		let limiter = RateLimiter { capacity: 2, ..RateLimiter::new(vec![]) };
		assert!(limiter.allow(ip("10.0.0.1"), false));
		assert!(limiter.allow(ip("10.0.0.2"), false));
		assert!((0..WRITES_PER_WINDOW).all(|_| limiter.allow(ip("10.0.0.3"), false)));
		assert!(limiter.allow(ip("10.0.0.3"), false), "a visitor with no room goes untracked");
		assert_eq!(limiter.windows.lock().map(|windows| windows.len()).ok(), Some(2));
		assert!((1..WRITES_PER_WINDOW).all(|_| limiter.allow(ip("10.0.0.1"), false)));
		assert!(!limiter.allow(ip("10.0.0.1"), false), "known visitors keep their window");
	}

	#[test]
	fn caps_the_streams_each_visitor_holds() {
		let limiter = Arc::new(RateLimiter::new(vec![]));
		let alice = ip("2001:db8:1:2::1");
		let slots = (0..STREAMS_PER_VISITOR)
			.map_while(|_| limiter.open_stream(alice))
			.collect::<Vec<_>>();
		assert_eq!(slots.len(), STREAMS_PER_VISITOR);
		assert!(limiter.open_stream(ip("2001:db8:1:2::2")).is_none(), "same /64");
		assert!(limiter.open_stream(ip("10.0.0.1")).is_some(), "another visitor is unaffected");
		drop(slots);
		assert!(limiter.open_stream(alice).is_some(), "closed streams are given back");
		assert_eq!(limiter.streams.lock().map(|streams| streams.len()).ok(), Some(0));
	}

	#[test]
	fn believes_forwarded_addresses_only_from_trusted_proxies() -> anyhow::Result<()> {
		let limiter = RateLimiter::new(RateLimiter::trusted_proxies("172.16.0.0/12, 127.0.0.1")?);
		let mut headers = HeaderMap::new();
		headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8, 127.0.0.1".parse()?);

		let direct = "203.0.113.9:5000".parse()?;
		assert_eq!(limiter.visitor(&headers, direct), ip("203.0.113.9"), "a client is not a proxy");
		let proxied = "172.18.0.1:5000".parse()?;
		assert_eq!(limiter.visitor(&headers, proxied), ip("5.6.7.8"), "trusted hops are skipped");
		assert_eq!(limiter.visitor(&HeaderMap::new(), proxied), ip("172.18.0.1"));
		headers.insert("x-forwarded-for", "5.6.7.8, junk, 127.0.0.1".parse()?);
		assert_eq!(limiter.visitor(&headers, proxied), ip("127.0.0.1"), "junk is not followed");
		assert!(RateLimiter::trusted_proxies("not an address").is_err());
		Ok(())
	}
}
