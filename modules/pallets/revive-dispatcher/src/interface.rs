use polkadot_sdk::*;

use pallet_revive::precompiles::alloy;

alloy::sol! {
	#![sol(all_derives)]
	/// @dev The on-chain address of the dispatcher precompile.
	address constant DISPATCHER_PRECOMPILE_ADDRESS = address(0xD270000);

	// Outcome of a successfully verified merkle-patricia proof
	struct StorageValue {
		// the storage key
		bytes key;
		// the encoded value
		bytes value;
	}

	struct ResponseReceipt {
		// commitment of the response object
		bytes32 responseCommitment;
		// address of the relayer responsible for this response delivery
		address relayer;
	}


	struct PostRequest {
		// the source state machine of this request
		bytes source;
		// the destination state machine of this request
		bytes dest;
		// request nonce
		uint64 nonce;
		// Module Id of this request origin
		bytes from;
		// destination module id
		bytes to;
		// timestamp by which this request times out.
		uint64 timeoutTimestamp;
		// request body
		bytes body;
	}

	struct GetRequest {
		// the source state machine of this request
		bytes source;
		// the destination state machine of this request
		bytes dest;
		// request nonce
		uint64 nonce;
		// Module Id of this request origin
		address from;
		// timestamp by which this request times out.
		uint64 timeoutTimestamp;
		// Storage keys to read.
		bytes[] keys;
		// height at which to read destination state machine
		uint64 height;
		// Some application-specific metadata relating to this request
		bytes context;
	}

	struct GetResponse {
		// The request that initiated this response
		GetRequest request;
		// storage values for get response
		StorageValue[] values;
	}

	struct PostResponse {
		// The request that initiated this response
		PostRequest request;
		// bytes for post response
		bytes response;
		// timestamp by which this response times out.
		uint64 timeoutTimestamp;
	}

	// @notice An object for dispatching post requests to the Hyperbridge
	struct DispatchPost {
		// bytes representation of the destination state machine
		bytes dest;
		// the destination module
		bytes to;
		// the request body
		bytes body;
		// timeout for this request in seconds
		uint64 timeout;
		// the amount put up to be paid to the relayer,
		// this is charged in `IDispatcher.feeToken` to `msg.sender`
		uint256 fee;
		// who pays for this request?
		address payer;
	}

	// @notice An object for dispatching get requests to the Hyperbridge
	struct DispatchGet {
		// bytes representation of the destination state machine
		bytes dest;
		// height at which to read the state machine
		uint64 height;
		// storage keys to read
		bytes[] keys;
		// timeout for this request in seconds
		uint64 timeout;
		// Hyperbridge protocol fees for processing this request.
		uint256 fee;
		// Some application-specific metadata relating to this request
		bytes context;
	}

	struct DispatchPostResponse {
		// The request that initiated this response
		PostRequest request;
		// bytes for post response
		bytes response;
		// timeout for this response in seconds
		uint64 timeout;
		// the amount put up to be paid to the relayer,
		// this is charged in `IDispatcher.feeToken` to `msg.sender`
		uint256 fee;
		// who pays for this request?
		address payer;
	}


	/**
	* @title The Dispatcher Interface
	* @author Polytope Labs (hello@polytope.technology)
	*
	* @notice The Dispatcher interface provides the core functionality for dispatching
	* interoperable messages through the ISMP protocol.
	*
	* @dev The Dispatcher provides the interfaces applications use for dispatching requests
	* and responses. This implementation focuses solely on outgoing message dispatch functionality,
	* delegating all verification logic to the IHandler contract.
	*/
	interface IDispatcher {
		/**
		 * @return the host state machine id
		 */
		function host() external view returns (bytes memory);

		/**
		 * @return the state machine identifier for the connected hyperbridge instance
		 */
		function hyperbridge() external view returns (bytes memory);

		/**
		 * @dev Returns the nonce immediately available for requests
		 * @return the `nonce`
		 */
		function nonce() external view returns (uint256);

		/**
		 * @dev Returns the address of the ERC-20 fee token contract configured for this state machine.
		 *
		 * @notice Hyperbridge collects it's dispatch fees in the provided token denomination. This will typically be in stablecoins.
		 *
		 * @return feeToken - The ERC20 contract address for fees.
		 */
		function feeToken() external view returns (address);

		/**
		 * @dev Returns the address of the per byte fee configured for the destination state machine.
		 *
		 * @notice Hyperbridge collects it's dispatch fees per every byte of the outgoing message.
		 *
		 * @param dest - The destination chain for the per byte fee.
		 * @return perByteFee - The per byte fee for outgoing messages.
		 */
		function perByteFee(bytes memory dest) external view returns (uint256);

		/**
		 * @dev Dispatch a POST request to Hyperbridge
		 *
		 * @notice Payment for the request can be made with either the native token or the IDispatcher.feeToken.
		 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
		 * Will revert if enough native tokens are not provided.
		 *
		 * If no native tokens are provided then it will try to collect payment from the calling contract in
		 * the IDispatcher.feeToken.
		 *
		 * @param request - post request
		 * @return commitment - the request commitment
		 */
		function dispatch(DispatchPost memory request) external payable returns (bytes32 commitment);

		/**
		 * @dev Dispatch a GET request to Hyperbridge
		 *
		 * @notice Payment for the request can be made with either the native token or the IDispatcher.feeToken.
		 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
		 * Will revert if enough native tokens are not provided.
		 *
		 * If no native tokens are provided then it will try to collect payment from the calling contract in
		 * the IDispatcher.feeToken.
		 *
		 * @param request - get request
		 * @return commitment - the request commitment
		 */
		function dispatch(DispatchGet memory request) external payable returns (bytes32 commitment);

		/**
		 * @dev Dispatch a POST response to Hyperbridge
		 *
		 * @notice Payment for the request can be made with either the native token or the IDispatcher.feeToken.
		 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
		 * Will revert if enough native tokens are not provided.
		 *
		 * If no native tokens are provided then it will try to collect payment from the calling contract in
		 * the IDispatcher.feeToken.
		 *
		 * @param response - post response
		 * @return commitment - the request commitment
		 */
		function dispatch(DispatchPostResponse memory response) external payable returns (bytes32 commitment);

		/**
		 * @dev Increase the relayer fee for a previously dispatched request.
		 * This is provided for use only on pending requests, such that when they timeout,
		 * the user can recover the entire relayer fee.
		 *
		 * @notice Payment can be made with either the native token or the IDispatcher.feeToken.
		 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
		 * Will revert if enough native tokens are not provided.
		 *
		 * If no native tokens are provided then it will try to collect payment from the calling contract in
		 * the IDispatcher.feeToken.
		 *
		 * If called on an already delivered request, these funds will be seen as a donation to the hyperbridge protocol.
		 * @param commitment - The request commitment
		 * @param amount - The amount provided in `IDispatcher.feeToken()`
		 */
		function fundRequest(bytes32 commitment, uint256 amount) external payable;

		/**
		 * @dev Increase the relayer fee for a previously dispatched response.
		 * This is provided for use only on pending responses, such that when they timeout,
		 * the user can recover the entire relayer fee.
		 *
		 * @notice Payment can be made with either the native token or the IDispatcher.feeToken.
		 * If native tokens are supplied, it will perform a swap under the hood using the local uniswap router.
		 * Will revert if enough native tokens are not provided.
		 *
		 * If no native tokens are provided then it will try to collect payment from the calling contract in
		 * the IDispatcher.feeToken.
		 *
		 * If called on an already delivered response, these funds will be seen as a donation to the hyperbridge protocol.
		 * @param commitment - The response commitment
		 * @param amount - The amount to be provided in `IDispatcher.feeToken()`
		 */
		function fundResponse(bytes32 commitment, uint256 amount) external payable;
	}

}
