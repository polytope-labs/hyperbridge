use futures::{
	channel::{mpsc, oneshot},
	SinkExt, StreamExt,
};
use std::future::Future;

/// Starts a pipelined task with the provided handler. The sender [`PipelineQueue`] always blocks
/// until the pipelined task is completed.
pub fn start_pipeline<T, F, R, Fut>(handler: F) -> PipelineQueue<T, R>
where
	T: Send + Sync + 'static,
	R: Send + Sync + 'static,
	F: FnMut(T) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = R> + Send + 'static,
{
	let (sender, rx) = mpsc::channel::<(T, oneshot::Sender<R>)>(64);

	let task = rx.for_each(move |(item, sender)| {
		let mut f = handler.clone();
		async move {
			let res = f(item).await;
			let _ = sender.send(res);
		}
	});

	tokio::spawn(task);

	PipelineQueue { sender }
}

/// Abstraction for dealing with pipelined tasks.
pub struct PipelineQueue<T, R> {
	/// Sending end of the pipeline
	sender: mpsc::Sender<(T, oneshot::Sender<R>)>,
}

impl<T, R> Clone for PipelineQueue<T, R> {
	fn clone(&self) -> Self {
		Self { sender: self.sender.clone() }
	}
}

impl<T, R> PipelineQueue<T, R> {
	/// Send a unit of work to the pipeline.
	pub async fn send(&self, item: T) -> anyhow::Result<R> {
		let (tx, rx) = oneshot::channel();

		self.sender.clone().send((item, tx)).await?;

		// wait for the task to complete
		let res = rx.await?;

		Ok(res)
	}
}
