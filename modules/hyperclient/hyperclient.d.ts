/* tslint:disable */
/* eslint-disable */
/**
 */
export function start(): void;

interface IConfig {
  // confuration object for the source chain
  source: IChainConfig;
  // confuration object for the destination chain
  dest: IChainConfig;
  // confuration object for hyperbridge
  hyperbridge: IHyperbridgeConfig;
  // Indexer url
  indexer: string;
}

interface IChainConfig {
  // rpc url of the chain
  rpc_url: string;
  // state machine identifier as a string
  state_machine: string;
  // contract address of the `IsmpHost` on this chain
  host_address: string;
  // contract address of the `IHandler` on this chain
  handler_address: string;
  // consensus state identifier of this chain on hyperbridge
  consensus_state_id: string;
}

interface IHyperbridgeConfig {
  // websocket rpc endpoint for hyperbridge
  rpc_url: string;
}

interface IPostRequest {
  // The source state machine of this request.
  source: string;
  // The destination state machine of this request.
  dest: string;
  // Module Id of the sending module
  from: string;
  // Module ID of the receiving module
  to: string;
  // The nonce of this request on the source chain
  nonce: bigint;
  // Encoded request body.
  data: string;
  // Timestamp which this request expires in seconds.
  timeoutTimestamp: bigint;
  // Height at which this request was emitted on the source
  height: bigint;
}

interface IPostResponse {
  // The request that triggered this response.
  post: IPostRequest;
  // The response message.
  response: Uint8Array;
  // Timestamp at which this response expires in seconds.
  timeoutTimestamp: bigint;
}

type MessageStatus =
  | Pending
  | SourceFinalized
  | HyperbridgeDelivered
  | HyperbridgeFinalized
  | DestinationDelivered
  | Timeout;

// This transaction is still pending on the source chain
interface Pending {
  kind: "Pending";
}

// This event is emitted on hyperbridge
interface SourceFinalized {
  kind: "SourceFinalized";
}

// This event is emitted on hyperbridge
interface HyperbridgeDelivered {
  kind: "HyperbridgeDelivered";
}

// This event is emitted on the destination chain
interface HyperbridgeFinalized {
  kind: "HyperbridgeFinalized";
}

// This event is emitted on the destination chain
interface DestinationDelivered {
  kind: "DestinationDelivered";
}

// The request has now timed out
interface Timeout {
  kind: "Timeout";
}

// The request has now timed out
interface DestinationFinalized {
  kind: "DestinationFinalized";
}

// The request has now timed out
interface HyperbridgeTimedout {
  kind: "HyperbridgeTimedout";
}

// The request has now timed out
interface HyperbridgeTimedout {
  kind: "HyperbridgeTimedout";
}

// The possible states of an inflight request
type MessageStatusWithMeta =
  | SourceFinalizedWithMetadata
  | HyperbridgeDeliveredWithMetadata
  | HyperbridgeFinalizedWithMetadata
  | DestinationDeliveredWithMetadata
  | Timeout
  | ErrorWithMetadata;

// The possible states of a timed-out request
type TimeoutStatusWithMeta =
  | DestinationFinalizedWithMetadata
  | HyperbridgeTimedoutWithMetadata
  | HyperbridgeFinalizedWithMetadata
  | TimeoutMessage
  | ErrorWithMetadata;

// This event is emitted on hyperbridge
interface SourceFinalizedWithMetadata {
  kind: "SourceFinalized";
  // Block height of the source chain that was finalized.
  finalized_height: bigint;
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
}

// This event is emitted on hyperbridge
interface HyperbridgeDeliveredWithMetadata {
  kind: "HyperbridgeDelivered";
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
}

// This event is emitted on the destination chain
interface HyperbridgeFinalizedWithMetadata {
  kind: "HyperbridgeFinalized";
  // Block height of hyperbridge chain that was finalized.
  finalized_height: bigint;
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
  // The transaction calldata which can be used for self-relay
  calldata: Uint8Array;
}

// This event is emitted on hyperbridge
interface HyperbridgeTimedoutWithMetadata {
  kind: "HyperbridgeTimedout";
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
}

// This event is emitted on the destination chain
interface DestinationDeliveredWithMetadata {
  kind: "DestinationDelivered";
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
}

// This event is emitted on the destination chain
interface TimeoutMessage {
  kind: "TimeoutMessage";
  // encoded call for HandlerV1.handlePostRequestTimeouts
  calldata: Uint8Array;
}

// This event is emitted on hyperbridge
interface DestinationFinalizedWithMetadata {
  kind: "DestinationFinalized";
  // The hash of the block where the event was emitted
  block_hash: `0x{string}`;
  // The hash of the extrinsic responsible for the event
  transaction_hash: `0x{string}`;
  // The block number where the event was emitted
  block_number: bigint;
}

// An error was encountered in the stream, the stream will come to an end.
interface ErrorWithMetadata {
  kind: "Error";
  // error description
  description: string;
}

/**
 * The hyperclient, allows the clients of hyperbridge to manage their in-flight ISMP requests
 * across multiple chains.
 */
export class HyperClient {
  free(): void;
  /**
   * Initialize the hyperclient
   * @param {IConfig} config
   * @returns {Promise<HyperClient>}
   */
  static init(config: IConfig): Promise<HyperClient>;
  /**
   * Queries the status of a request and returns `MessageStatusWithMetadata`
   * @param {IPostRequest} request
   * @returns {Promise<MessageStatus>}
   */
  query_request_status(request: IPostRequest): Promise<MessageStatus>;
  /**
   * Accepts a post response and returns a `MessageStatusWithMetadata`
   * @param {IPostResponse} response
   * @returns {Promise<MessageStatus>}
   */
  query_response_status(response: IPostResponse): Promise<MessageStatus>;
  /**
   * Return the status of a post request as a `ReadableStream` that yields
   * `MessageStatusWithMeta`
   * @param {IPostRequest} request
   * @returns {Promise<ReadableStream<MessageStatusWithMeta>>}
   */
  request_status_stream(
    request: IPostRequest,
  ): Promise<ReadableStream<MessageStatusWithMeta>>;
  /**
   * Given a post request that has timed out returns a `ReadableStream` that yields a
   * `TimeoutStatus` This function will not check if the request has timed out, only call it
   * when you receive a `MesssageStatus::TimeOut` from `query_request_status` or
   * `request_status_stream`. The stream ends when once it yields a `TimeoutMessage`
   * @param {IPostRequest} request
   * @returns {Promise<ReadableStream<TimeoutStatusWithMeta>>}
   */
  timeout_post_request(
    request: IPostRequest,
  ): Promise<ReadableStream<TimeoutStatusWithMeta>>;
  /**
   * @returns {string | undefined}
   */
  get_indexer_url(): string | undefined;
}
/**
 */
export class IntoUnderlyingByteSource {
  free(): void;
  /**
   * @param {ReadableByteStreamController} controller
   */
  start(controller: ReadableByteStreamController): void;
  /**
   * @param {ReadableByteStreamController} controller
   * @returns {Promise<any>}
   */
  pull(controller: ReadableByteStreamController): Promise<any>;
  /**
   */
  cancel(): void;
  /**
   */
  readonly autoAllocateChunkSize: number;
  /**
   */
  readonly type: string;
}
/**
 */
export class IntoUnderlyingSink {
  free(): void;
  /**
   * @param {any} chunk
   * @returns {Promise<any>}
   */
  write(chunk: any): Promise<any>;
  /**
   * @returns {Promise<any>}
   */
  close(): Promise<any>;
  /**
   * @param {any} reason
   * @returns {Promise<any>}
   */
  abort(reason: any): Promise<any>;
}
/**
 */
export class IntoUnderlyingSource {
  free(): void;
  /**
   * @param {ReadableStreamDefaultController} controller
   * @returns {Promise<any>}
   */
  pull(controller: ReadableStreamDefaultController): Promise<any>;
  /**
   */
  cancel(): void;
}
