use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use jsonrpsee::server::{
	serve_with_graceful_shutdown, stop_channel, HttpBody, HttpRequest, HttpResponse, Methods,
	Server, ServerHandle,
};
use tower::Service;

const MAX_BODY_SIZE: u32 = 32 * 1024 * 1024;

struct Routes {
	by_path: HashMap<String, Methods>,
	root: Option<Methods>,
}

impl Routes {
	fn new(chains: Vec<(String, Methods)>) -> Self {
		let root = (chains.len() == 1).then(|| chains[0].1.clone());
		let by_path = chains
			.into_iter()
			.map(|(name, methods)| (format!("/{name}"), methods))
			.collect();
		Self { by_path, root }
	}

	fn select<B>(&self, request: &HttpRequest<B>) -> Option<&Methods> {
		let path = request.uri().path().trim_end_matches('/');
		if path.is_empty() {
			return self.root.as_ref();
		}
		self.by_path.get(path)
	}
}

pub async fn serve(
	listen: SocketAddr,
	chains: Vec<(String, Methods)>,
) -> anyhow::Result<(SocketAddr, ServerHandle)> {
	let listener = tokio::net::TcpListener::bind(listen).await?;
	let local_addr = listener.local_addr()?;
	let routes = Arc::new(Routes::new(chains));
	let (stop_handle, server_handle) = stop_channel();
	let service_builder = Server::builder()
		.http_only()
		.max_request_body_size(MAX_BODY_SIZE)
		.max_response_body_size(MAX_BODY_SIZE)
		.to_service_builder();

	tokio::spawn(async move {
		loop {
			let (stream, _) = tokio::select! {
				accepted = listener.accept() => match accepted {
					Ok(conn) => conn,
					Err(err) => {
						tracing::warn!(%err, "failed to accept connection");
						continue;
					},
				},
				_ = stop_handle.clone().shutdown() => break,
			};

			let routes = routes.clone();
			let service_builder = service_builder.clone();
			let stop_handle = stop_handle.clone();
			let connection_stop = stop_handle.clone();
			let service = tower::service_fn(move |request| {
				let rpc = routes.select(&request).map(|methods| {
					service_builder.clone().build(methods.clone(), stop_handle.clone())
				});
				async move {
					match rpc {
						Some(mut rpc) => rpc.call(request).await,
						None => Ok(not_found()),
					}
				}
			});
			tokio::spawn(serve_with_graceful_shutdown(stream, service, connection_stop.shutdown()));
		}
	});

	Ok((local_addr, server_handle))
}

fn not_found() -> HttpResponse {
	HttpResponse::builder()
		.status(404)
		.header("content-type", "text/plain")
		.body(HttpBody::from("unknown chain path\n"))
		.expect("static response")
}
