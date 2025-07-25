use anyhow::anyhow;
use axum::{
	extract::{Path, State as AxumState},
	http::StatusCode,
	response::{IntoResponse, Response},
	routing, Json,
};
use ismp::host::StateMachine;
use primitive_types::H160;
use socketioxide::{
	extract::{Data, SocketRef, State},
	socket::{DisconnectReason, Sid},
	SocketIo,
};
use sp_core::{bytes::from_hex, ecdsa, ByteArray, Encode, Pair};
use std::{collections::BTreeMap, env, str::FromStr, sync::Arc};
use telemetry_server::Message;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::FmtSubscriber;

/// Tracking the addresses of all online relayers
type SharedAppState = Arc<RwLock<AppState>>;

#[derive(Default)]
struct AppState {
	/// map of networks to active relayers
	networks: BTreeMap<StateMachine, BTreeMap<H160, bool>>,
	// active clients
	clients: BTreeMap<Sid, Vec<(StateMachine, H160)>>,
}

impl AppState {
	fn insert(&mut self, id: Sid, data: Message) {
		for (network, address) in data.metadata.clone() {
			self.networks
				.entry(network)
				.and_modify(|map| {
					map.insert(address, true);
				})
				.or_insert(BTreeMap::from([(address, true)]));
		}
		self.clients.insert(id, data.metadata);
	}

	fn remove(&mut self, id: Sid) {
		if let Some(metadata) = self.clients.remove(&id) {
			for (network, address) in metadata {
				self.networks.entry(network).and_modify(|map| {
					map.remove(&address);
				});
			}
		}
	}
}

/// Called whenever a new client, who is ideally a realyer is connected.
async fn on_connect(socket: SocketRef, Data(data): Data<Message>, state: State<SharedAppState>) {
	tracing::info!("socket connected {}", socket.id);

	let bytes = from_hex(
		option_env!("TELEMETRY_SECRET_KEY")
			.unwrap_or("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
	)
	.expect("TELEMETRY_SECRET_KEY should be valid hex!");
	let pair =
		ecdsa::Pair::from_seed_slice(&bytes).expect("TELEMETRY_SECRET_KEY must be 64 chars!");
	let signature = ecdsa::Signature::from_slice(data.signature.as_slice())
		.map(|signature| ecdsa::Pair::verify(&signature, data.metadata.encode(), &pair.public()));

	if matches!(signature, Ok(true)) {
		tracing::info!("Authenticated {} successfully", socket.id);
		// add to global state
		state.write().await.insert(socket.id, data);
	} else {
		tracing::info!("Disconnecting unauthorized {}", socket.id);
		// invalid signatures get dropped
		socket.disconnect().ok();
		return;
	}

	socket.on_disconnect(
		|socket: SocketRef, reason: DisconnectReason, state: State<SharedAppState>| async move {
			tracing::info!(
				"Socket {} on ns {} disconnected, reason: {:?}",
				socket.id,
				socket.ns(),
				reason
			);
			// remove from global state
			state.write().await.remove(socket.id);
		},
	);
}

async fn online(
	Path(network): Path<String>,
	state: AxumState<SharedAppState>,
) -> Result<Json<Vec<H160>>, AppError> {
	let network = StateMachine::from_str(&network).map_err(|err| anyhow!("{err}"))?;
	let relayers = state
		.read()
		.await
		.networks
		.get(&network)
		.map(|map| map.keys().cloned().collect::<Vec<_>>())
		.unwrap_or(vec![]);

	Ok(Json(relayers))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	tracing::subscriber::set_global_default(FmtSubscriber::default())?;

	let relayers = SharedAppState::default();
	let (layer, io) = SocketIo::builder().with_state(relayers.clone()).build_layer();

	io.ns("/", on_connect);

	let combined = ServiceBuilder::new().layer(CorsLayer::permissive()).layer(layer);
	let app = axum::Router::new()
		.route("/:network", routing::get(online))
		.layer(combined)
		.with_state(relayers);

	let port = env::var("PORT").ok().unwrap_or("3000".into());
	tracing::info!("Starting server on port: {port}");
	// run our app with hyper, listening globally on port 3000
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, app).await?;

	Ok(())
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		(StatusCode::INTERNAL_SERVER_ERROR, format!("Something went wrong: {}", self.0))
			.into_response()
	}
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
	E: Into<anyhow::Error>,
{
	fn from(err: E) -> Self {
		Self(err.into())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rust_socketio::ClientBuilder;
	use sp_core::ByteArray;
	use std::{thread, time::Duration};

	#[test]
	#[ignore]
	fn test_socket_io_client() -> Result<(), anyhow::Error> {
		let bytes = from_hex(
			option_env!("TELEMETRY_SECRET_KEY")
				.unwrap_or("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
		)
		.expect("TELEMETRY_SECRET_KEY should be valid hex!");
		let pair =
			ecdsa::Pair::from_seed_slice(&bytes).expect("TELEMETRY_SECRET_KEY must be 64 chars!");
		let mut message =
			Message { signature: vec![], metadata: vec![(StateMachine::Evm(97), H160::random())] };
		message.signature = pair.sign(message.metadata.encode().as_slice()).to_raw_vec();
		// get a socket that is connected to the admin namespace
		let socket = ClientBuilder::new("http://localhost:3000")
			.namespace("/")
			.auth(serde_json::to_value(message.clone())?)
			.reconnect(true)
			.reconnect_on_disconnect(true)
			.max_reconnect_attempts(255)
			.on("error", |err, _| println!("Error: {:#?}", err))
			.on("connected", |err, _| println!("Connected: {:#?}", err))
			.connect()
			.expect("Connection failed");

		thread::sleep(Duration::from_secs(1000));

		socket.disconnect()?;

		Ok(())
	}
}
