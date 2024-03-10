/* tslint:disable */
/* eslint-disable */
/**
* Functions takes in a post request and returns one of the following json strings variants
* Status variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
* `DestinationDelivered`, `Timeout`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @returns {Promise<any>}
*/
export function query_request_status(request: JsPost, config_js: JsClientConfig): Promise<any>;
/**
* Function takes in a post response and returns one of the following json strings variants
* Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
* `DestinationDelivered`, `Timeout`
* @param {JsResponse} response
* @param {JsClientConfig} config_js
* @returns {Promise<any>}
*/
export function query_response_status(response: JsResponse, config_js: JsClientConfig): Promise<any>;
/**
* Accepts a post request that has timed out returns a stream that yields the following json
* strings variants Status Variants: `Pending`, `DestinationFinalized`, `HyperbridgeTimedout`,
* `HyperbridgeFinalized`, `{ "TimeoutMessage": [...] }`. This function will not check if the
* request has timed out, only call it when sure that the request has timed out after calling
* `query_request_status`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @returns {Promise<ReadableStream>}
*/
export function timeout_post_request(request: JsPost, config_js: JsClientConfig): Promise<ReadableStream>;
/**
* Returns a stream that yields the following json
* strings variants Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`,
* `HyperbridgeFinalized`, `DestinationDelivered`, `Timeout`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @returns {Promise<ReadableStream>}
*/
export function request_status_stream(request: JsPost, config_js: JsClientConfig): Promise<ReadableStream>;
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
/**
*/
export class JsChainConfig {
  free(): void;
/**
*/
  consensus_state_id: Uint8Array;
/**
*/
  handler_address: Uint8Array;
/**
*/
  host_address: Uint8Array;
/**
*/
  rpc_url: string;
/**
*/
  state_machine: string;
}
/**
*/
export class JsClientConfig {
  free(): void;
/**
*/
  dest: JsChainConfig;
/**
*/
  hyperbridge: JsHyperbridgeConfig;
/**
*/
  source: JsChainConfig;
}
/**
*/
export class JsHyperbridgeConfig {
  free(): void;
/**
*/
  rpc_url: string;
}
/**
*/
export class JsPost {
  free(): void;
/**
* Encoded Request.
*/
  data: Uint8Array;
/**
* The destination state machine of this request.
*/
  dest: string;
/**
* Module Id of the sending module
*/
  from: Uint8Array;
/**
* Gas limit for executing the request on destination
* This value should be zero if destination module is not a contract
*/
  gas_limit: bigint;
/**
* Height at which this request was emitted on the source chain
*/
  height: bigint;
/**
* The nonce of this request on the source chain
*/
  nonce: bigint;
/**
* The source state machine of this request.
*/
  source: string;
/**
* Timestamp which this request expires in seconds.
*/
  timeout_timestamp: bigint;
/**
* Module ID of the receiving module
*/
  to: Uint8Array;
}
/**
*/
export class JsResponse {
  free(): void;
/**
* Gas limit for executing the response on destination, only used for solidity modules.
*/
  gas_limit: bigint;
/**
* The request that triggered this response.
*/
  post: JsPost;
/**
* The response message.
*/
  response: Uint8Array;
/**
* Timestamp at which this response expires in seconds.
*/
  timeout_timestamp: bigint;
}
