use jsonrpsee::types::ErrorObjectOwned;

use crate::blocks::BlockError;

pub const SERVER_ERROR: i32 = -32000;
pub const INVALID_PARAMS: i32 = -32602;

pub fn to_rpc_error(e: BlockError) -> ErrorObjectOwned {
	match e {
		BlockError::RangeTooLarge { from, max, .. } => {
			let last = from.saturating_add(max.saturating_sub(1));
			server_error(format!("this block range should work: [{from:#x}, {last:#x}]"))
		},
		BlockError::BeyondHead { to, head } =>
			server_error(format!("toBlock {to:#x} is beyond finalized head {head:#x}")),
		BlockError::InvalidRange { .. } => invalid_params(e.to_string()),
		other => server_error(other.to_string()),
	}
}

pub fn server_error(message: impl Into<String>) -> ErrorObjectOwned {
	ErrorObjectOwned::owned(SERVER_ERROR, message, None::<()>)
}

pub fn invalid_params(message: impl Into<String>) -> ErrorObjectOwned {
	ErrorObjectOwned::owned(INVALID_PARAMS, message, None::<()>)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::substrate::RpcError;

	#[test]
	fn range_too_large_tells_rindexer_the_working_range() {
		let err = to_rpc_error(BlockError::RangeTooLarge { from: 100, to: 102, max: 2 });
		assert_eq!(err.code(), SERVER_ERROR);
		assert_eq!(err.message(), "this block range should work: [0x64, 0x65]");

		let err = to_rpc_error(BlockError::RangeTooLarge { from: 0, to: 5000, max: 1000 });
		assert_eq!(err.message(), "this block range should work: [0x0, 0x3e7]");
	}

	#[test]
	fn beyond_head_and_invalid_range() {
		let err = to_rpc_error(BlockError::BeyondHead { to: 103, head: 102 });
		assert_eq!(err.code(), SERVER_ERROR);
		assert_eq!(err.message(), "toBlock 0x67 is beyond finalized head 0x66");

		let err = to_rpc_error(BlockError::InvalidRange { from: 102, to: 101 });
		assert_eq!(err.code(), INVALID_PARAMS);
		assert_eq!(err.message(), "fromBlock 102 is after toBlock 101");
	}

	#[test]
	fn block_scoped_failures_keep_the_block_number() {
		let source = Box::new(BlockError::Rpc(RpcError::Transport("boom".into())));
		let err = to_rpc_error(BlockError::At { number: 101, source });
		assert_eq!(err.code(), SERVER_ERROR);
		assert_eq!(err.message(), "block 101: rpc transport: boom");

		let err = to_rpc_error(BlockError::MissingEvents(7));
		assert_eq!(err.message(), "missing System.Events at block 7");
	}
}
