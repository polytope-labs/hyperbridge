use std::{collections::HashMap, sync::Arc, time::Duration};

use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Request, RequestResponse, Response},
	util::{hash_request, hash_response},
};
use tesseract_client::AnyClient;
use tesseract_primitives::{
	config::{Chain, RelayerConfig},
	Hasher, HyperbridgeClaim, IsmpProvider, Query,
};
use transaction_fees::TransactionPayment;

use crate::{
	events::{chunk_size, return_successful_queries},
	FeeAccSender,
};

pub fn spawn_unprofitable_retries_task<
	A: IsmpProvider + Clone + 'static,
	B: IsmpProvider + Clone + HyperbridgeClaim + 'static,
>(
	dest: A,
	hyperbridge: B,
	client_map: HashMap<StateMachine, AnyClient>,
	tx_payment: Arc<TransactionPayment>,
	config: RelayerConfig,
	coprocessor: Chain,
	fee_acc_sender: FeeAccSender,
) -> Result<(), anyhow::Error> {
	tokio::spawn(async move {
		// Default to every 10 minutes
		let mut interval = tokio::time::interval(Duration::from_secs(
			config.unprofitable_retry_frequency.unwrap_or(10 * 60),
		));
		loop {
			interval.tick().await;
			let unprofitables =
				match tx_payment.unprofitable_messages(&dest.state_machine_id().state_id).await {
					Ok(messages) => messages,
					Err(_) => {
						continue;
					},
				};

			if !unprofitables.is_empty() {
				tracing::trace!("Starting retries of previously unprofitable messages");
				let mut request_messages = vec![];
				let mut response_messages = vec![];
				let mut ids = vec![];
				let mut request_queries = vec![];
				let mut response_queries = vec![];
				let mut post_requests = vec![];
				let mut post_responses = vec![];
				// Store the highest proof height in this variable
				let mut state_machine_height: Option<StateMachineHeight> = None;
				unprofitables.into_iter().for_each(|(message, id)| {
					match message {
					Message::Request(msg) => {
						let post = msg.requests.get(0).cloned().expect(
							"Inconsistent Database, withdraw all fees and  restart relayer with a fresh database",
						);
						let query = {
							let req = Request::Post(post.clone());
							let hash = hash_request::<Hasher>(&req);

							Query {
								source_chain: req.source_chain(),
								dest_chain: req.dest_chain(),
								nonce: req.nonce(),
								commitment: hash,
							}
						};
						if let Some(state_machine_height) = state_machine_height.as_mut() {
							if msg.proof.height.height > state_machine_height.height {
								*state_machine_height = msg.proof.height
							}
						} else {
							state_machine_height = Some(msg.proof.height)
						}
						post_requests.push(post);
						request_messages.push(Message::Request(msg));
						request_queries.push(query);
						ids.push(id);
					},
					Message::Response(msg) => match &msg.datagram {
						ismp::router::RequestResponse::Response(responses) => {
							let post_response = match responses.get(0).cloned().expect(
								"Inconsistent Database, withdraw all fees and restart relayer with a fresh database",
							) {
								Response::Post(post_response) => post_response,
								Response::Get(_) =>
									panic!("Inconsistent Db, withdraw all fees and restart relayer with a fresh database"),
							};
							let resp = Response::Post(post_response);
							let hash = hash_response::<Hasher>(&resp);

							let query = Query {
								source_chain: resp.source_chain(),
								dest_chain: resp.dest_chain(),
								nonce: resp.nonce(),
								commitment: hash,
							};
							if let Some(state_machine_height) = state_machine_height.as_mut() {
								if msg.proof.height.height > state_machine_height.height {
									*state_machine_height = msg.proof.height
								}
							} else {
								state_machine_height = Some(msg.proof.height)
							}
							post_responses.push(resp);
							response_messages.push(Message::Response(msg));
							response_queries.push(query);
							ids.push(id);
						},
						_ => panic!("Inconsistent Db, withdraw all fees and restart relayer with a fresh database"),
					},
					_ => panic!("Inconsistent Db, withdraw all fees and restart relayer with a fresh database"),
				}
				});

				let mut outgoing_messages = vec![];
				let mut new_unprofitable_messages = vec![];
				match return_successful_queries(
					&dest,
					request_messages,
					request_queries,
					config.minimum_profit_percentage,
					coprocessor,
					&client_map,
				)
				.await
				{
					Ok(request_profitablility) => {
						let post_requests: Vec<_> = post_requests
							.into_iter()
							.zip(request_profitablility.queries.iter())
							.filter_map(|(current_post, current_query)| {
								if current_query.is_some() {
									Some(current_post)
								} else {
									None
								}
							})
							.collect();

						let successful_queries: Vec<Query> = request_profitablility
							.queries
							.into_iter()
							.filter_map(|query| query)
							.collect();

						if !successful_queries.is_empty() {
							tracing::trace!(
								"Unprofitable Messages Retries: Querying request proof for batch length {}",
								successful_queries.len()
							);
							let chunks = chunk_size(dest.state_machine_id().state_id);
							let query_chunks = successful_queries.chunks(chunks);
							let post_request_chunks = post_requests.chunks(chunks);
							for (queries, post_requests) in
								query_chunks.into_iter().zip(post_request_chunks)
							{
								if let Some(state_machine_height) = state_machine_height {
									if let Ok(requests_proof) = hyperbridge
										.query_requests_proof(
											state_machine_height.height,
											queries.to_vec(),
										)
										.await
									{
										let msg = RequestMessage {
											requests: post_requests.to_vec(),
											proof: Proof {
												height: state_machine_height,
												proof: requests_proof,
											},
											signer: dest.address(),
										};
										outgoing_messages.push(Message::Request(msg));
									}
								}
							}
						}

						new_unprofitable_messages.extend(request_profitablility.unprofitable_msgs);
					},
					Err(err) => {
						tracing::error!(
							"Unprofitable Messages Retries: Debug tracing failed: {err:?}"
						)
					},
				}

				match return_successful_queries(
					&dest,
					response_messages,
					response_queries,
					config.minimum_profit_percentage,
					coprocessor,
					&client_map,
				)
				.await
				{
					Ok(response_profitablility) => {
						let post_responses: Vec<_> = post_responses
							.into_iter()
							.zip(response_profitablility.queries.iter())
							.filter_map(|(current_resp, current_query)| {
								if current_query.is_some() {
									Some(current_resp)
								} else {
									None
								}
							})
							.collect();

						let successful_queries: Vec<Query> = response_profitablility
							.queries
							.into_iter()
							.filter_map(|query| query)
							.collect();

						if !successful_queries.is_empty() {
							tracing::trace!(
								"Unprofitable Messages Retries: Querying response proof for batch length {}",
								successful_queries.len()
							);
							let chunks = chunk_size(dest.state_machine_id().state_id);
							let query_chunks = successful_queries.chunks(chunks);
							let post_response_chunks = post_responses.chunks(chunks);
							for (queries, post_responses) in
								query_chunks.into_iter().zip(post_response_chunks)
							{
								if let Some(state_machine_height) = state_machine_height {
									if let Ok(responses_proof) = hyperbridge
										.query_responses_proof(
											state_machine_height.height,
											queries.to_vec(),
										)
										.await
									{
										let msg = ResponseMessage {
											datagram: RequestResponse::Response(
												post_responses.to_vec(),
											),
											proof: Proof {
												height: state_machine_height,
												proof: responses_proof,
											},
											signer: dest.address(),
										};
										outgoing_messages.push(Message::Response(msg));
									}
								}
							}
						}

						new_unprofitable_messages.extend(response_profitablility.unprofitable_msgs);
					},
					Err(err) => {
						tracing::error!(
							"Unprofitable Messages Retries: Debug tracing failed: {err:?}"
						)
					},
				}

				if !outgoing_messages.is_empty() {
					tracing::info!(
						target: "tesseract",
						"Unprofitable Messages Retries: 🛰️ Transmitting ismp messages from {} to {}", hyperbridge.name(), dest.name()
					);
					if let Ok(receipts) = dest.submit(outgoing_messages).await {
						if !receipts.is_empty() {
							// Store receipts in database before auto accumulation
							tracing::trace!(target: "tesseract", "Persisting {} deliveries from {}->{} to the db", receipts.len(), hyperbridge.name(), dest.name());
							if let Err(err) = tx_payment.store_messages(receipts.clone()).await {
								tracing::error!(
									"Failed to persist {} deliveries to database: {err:?}",
									receipts.len()
								)
							}
							// Send receipts to the fee accumulation task
							match fee_acc_sender.send(receipts).await {
								Err(_sent) => {
									tracing::error!(
										"Fee auto accumulation failed You can try again manually"
									)
								},
								_ => {},
							}
						}
					}
				}
				// Delete previous batch from db
				if !ids.is_empty() {
					tracing::trace!(target: "tesseract", "Unprofitable Messages Retries: Deleting some unprofitable messages from the Db");
					let _ = tx_payment.delete_unprofitable_messages(ids).await;
				}
				// Store the new batch
				if !new_unprofitable_messages.is_empty() {
					tracing::trace!(target: "tesseract", "Unprofitable Messages Retries: Persisting {} unprofitable messages going to {} to the db", new_unprofitable_messages.len(), dest.name());
					let _ = tx_payment
						.store_unprofitable_messages(
							new_unprofitable_messages,
							dest.state_machine_id().state_id,
						)
						.await;
				}
			}
		}
	});
	Ok::<_, anyhow::Error>(())
}
