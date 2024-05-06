use futures::{
	channel::{mpsc, oneshot},
	SinkExt, StreamExt,
};
use std::future::Future;

/// Starts a pipelined task with the provided handler. The sender [`PipelineQueue`] always blocks
/// until the pipelined task is completed.
pub fn start_pipeline<T, F, Fut>(handler: F) -> PipelineQueue<T>
where
	T: Send + Sync + 'static,
	F: FnMut(T) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = ()> + Send + 'static,
{
	let (sender, rx) = mpsc::channel::<(T, oneshot::Sender<()>)>(32);

	let task = rx.for_each(move |(item, sender)| {
		let mut f = handler.clone();
		async move {
			f(item).await;
			let _ = sender.send(());
		}
	});

	tokio::spawn(task);

	PipelineQueue { sender }
}

/// Abstraction for dealing with pipelined tasks.
pub struct PipelineQueue<T> {
	/// Sending end of the pipeline
	sender: mpsc::Sender<(T, oneshot::Sender<()>)>,
}

impl<T> Clone for PipelineQueue<T> {
	fn clone(&self) -> Self {
		Self { sender: self.sender.clone() }
	}
}

impl<T> PipelineQueue<T> {
	/// Send a unit of work to the pipeline.
	pub async fn send(mut self, item: T) -> anyhow::Result<()> {
		let (tx, rx) = oneshot::channel();

		self.sender.send((item, tx)).await?;

		// wait for the task to complete
		rx.await?;

		Ok(())
	}
}
