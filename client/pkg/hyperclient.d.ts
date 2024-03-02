/* tslint:disable */
/* eslint-disable */
/**
* Functions takes in a post request and returns a `MessageStatus`
* @param {any} request
* @param {any} config_js
* @returns {Promise<any>}
*/
export function query_request_status(request: any, config_js: any): Promise<any>;
/**
* Function takes in a post response and returns a `MessageStatus`
* @param {any} response
* @param {any} config_js
* @returns {Promise<any>}
*/
export function query_response_status(response: any, config_js: any): Promise<any>;
/**
* Accepts a post request that has timed out returns a stream that yields `TimeoutStatus`
* This function will not check if request has timed out, only call it when sure that the request
* has timed out after using `query_request_status`
* @param {any} request
* @param {any} config_js
* @returns {Promise<ReadableStream>}
*/
export function timeout_post_request(request: any, config_js: any): Promise<ReadableStream>;
/**
* Races between a timeout stream and request processing stream, and yields `MessageStatus`
* If it yields `MessageStatus::Timeout`, the consumer of the stream should handle it appropriately
* @param {any} request
* @param {any} config_js
* @param {bigint} post_request_height
* @returns {Promise<ReadableStream>}
*/
export function subscribe_to_request_status(request: any, config_js: any, post_request_height: bigint): Promise<ReadableStream>;
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
