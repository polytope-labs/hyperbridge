// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/* tslint:disable */
/* eslint-disable */
/**
 */
export function start(): void;

export type HexString = `0x{string}` | `0x${string}`;

export interface IConfig {
    // confuration object for the source chain
    source: IEvmConfig | ISubstrateConfig;
    // confuration object for the destination chain
    dest: IEvmConfig | ISubstrateConfig;
    // confuration object for hyperbridge
    hyperbridge: IHyperbridgeConfig;
    // Flag to enable tracing console logs
    tracing?: boolean;
}

export interface IEvmConfig {
    // rpc url of the chain
    rpc_url: string;
    // state machine identifier as a string
    state_machine: string;
    // contract address of the `IsmpHost` on this chain
    host_address: string;
    // consensus state identifier of this chain on hyperbridge
    consensus_state_id: string;
}

export interface ISubstrateConfig {
    // rpc url of the chain
    rpc_url: string;
    // consensus state identifier of this chain on hyperbridge
    consensus_state_id: string;
    // consensus state identifier of this chain on hyperbridge
    hash_algo: "Keccak" | "Blake2";
    // state machine identifier as a string
    state_machine: string;
}

export interface IHyperbridgeConfig {
    // websocket rpc endpoint for hyperbridge
    rpc_url: string;
    // state machine identifier as a string
    state_machine: string;
    // consensus state identifier of hyperbridge on the destination chain
    consensus_state_id: string;
}

export interface IPostRequest {
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
    body: string;
    // Timestamp which this request expires in seconds.
    timeoutTimestamp: bigint;
}

export interface IGetRequest {
    // The source state machine of this request.
    source: string;
    // The destination state machine of this request.
    dest: string;
    // Module Id of the sending module
    from: string;
    // The nonce of this request on the source chain
    nonce: bigint;
    // Height at which to read the state machine.
    height: bigint;
    /// Raw Storage keys that would be used to fetch the values from the counterparty
    /// For deriving storage keys for ink contract fields follow the guide in the link below
    /// `<https://use.ink/datastructures/storage-in-metadata#a-full-example>`
    /// The algorithms for calculating raw storage keys for different substrate pallet storage
    /// types are described in the following links
    /// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/map.rs#L34-L42>`
    /// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/double_map.rs#L34-L44>`
    /// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/nmap.rs#L39-L48>`
    /// `<https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/value.rs#L37>`
    /// For fetching keys from EVM contracts each key should be 52 bytes
    /// This should be a concatenation of contract address and slot hash
    keys: HexString[];
    // Timestamp which this request expires in seconds.
    timeoutTimestamp: bigint;
}

export interface IPostResponse {
    // The request that triggered this response.
    post: IPostRequest;
    // The response message.
    response: string;
    // Timestamp at which this response expires in seconds.
    timeoutTimestamp: bigint;
}

// This transaction is still pending on the source chain
export interface Pending {
    kind: "Pending";
}

// This event is emitted on hyperbridge
export interface SourceFinalized {
    kind: "SourceFinalized";
}

// This event is emitted on hyperbridge
export interface HyperbridgeVerified {
    kind: "HyperbridgeVerified";
}

// This event is emitted on the destination chain
export interface HyperbridgeFinalized {
    kind: "HyperbridgeFinalized";
}

// This event is emitted on the destination chain
export interface DestinationDelivered {
    kind: "DestinationDelivered";
}

// The request has now timed out
export interface Timeout {
    kind: "Timeout";
}

// The request has now timed out
export interface DestinationFinalized {
    kind: "DestinationFinalized";
}

// The request has now timed out
export interface HyperbridgeTimedout {
    kind: "HyperbridgeTimedout";
}

// The request has now timed out
export interface HyperbridgeTimedout {
    kind: "HyperbridgeTimedout";
}

// The request timeout has been finalized by the destination
export interface DestinationFinalizedState {
    // the height of the destination chain at which the time out was finalized
    DestinationFinalized: bigint;
}

// Hyperbridge has finalized some state
export interface HyperbridgeFinalizedState {
    // The height of the state commitment that was finalized
    HyperbridgeFinalized: bigint;
}

// The source chain has finalized some state commitment
export interface SourceFinalizedState {
    // The height of the source chain which was finalized
    SourceFinalized: bigint;
}

// The message has been verified & aggregated to Hyperbridge
export interface HyperbridgeVerifiedState {
    // Height at which the message was aggregated to Hyperbridge
    HyperbridgeVerified: bigint;
}

// Initial state for a pending cross-chain message
export interface MessageDispatched {
    // The height at which the message was dispatched from the source chain
    Dispatched: bigint;
}

// The possible initial states of a timeout (request or response) stream
export type TimeoutStreamState =
    | "Pending"
    | DestinationFinalizedState
    | HyperbridgeVerifiedState;

// The possible initial states of a message status (request or response) stream
export type MessageStatusStreamState =
    | MessageDispatched
    | SourceFinalizedState
    | HyperbridgeVerifiedState
    | HyperbridgeFinalizedState;

// The possible states of an inflight request
export type MessageStatusWithMeta =
    | Pending
    | SourceFinalizedWithMetadata
    | HyperbridgeVerifiedWithMetadata
    | HyperbridgeFinalizedWithMetadata
    | DestinationDeliveredWithMetadata
    | Timeout
    | ErrorWithMetadata;

// The possible states of a timed-out request
export type TimeoutStatusWithMeta =
    | DestinationFinalizedWithMetadata
    | HyperbridgeVerifiedWithMetadata
    | HyperbridgeFinalizedWithMetadata
    | ErrorWithMetadata;

// This event is emitted on hyperbridge
export interface SourceFinalizedWithMetadata {
    kind: "SourceFinalized";
    // Block height of the source chain that was finalized.
    finalized_height: bigint;
    // The hash of the block where the event was emitted
    block_hash: HexString;
    // The hash of the extrinsic responsible for the event
    transaction_hash: HexString;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on hyperbridge
export interface HyperbridgeVerifiedWithMetadata {
    kind: "HyperbridgeVerified";
    // The hash of the block where the event was emitted
    block_hash: HexString;
    // The hash of the extrinsic responsible for the event
    transaction_hash: HexString;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on the destination chain
export interface HyperbridgeFinalizedWithMetadata {
    kind: "HyperbridgeFinalized";
    // Block height of hyperbridge chain that was finalized.
    finalized_height: bigint;
    // The hash of the block where the event was emitted
    block_hash: HexString;
    // The hash of the extrinsic responsible for the event
    transaction_hash: HexString;
    // The block number where the event was emitted
    block_number: bigint;
    // The transaction calldata which can be used for self-relay
    calldata: HexString;
}

// This event is emitted on the destination chain
export interface DestinationDeliveredWithMetadata {
    kind: "DestinationDelivered";
    // The hash of the block where the event was emitted
    block_hash: HexString;
    // The hash of the extrinsic responsible for the event
    transaction_hash: HexString;
    // The block number where the event was emitted
    block_number: bigint;
}

// This event is emitted on the destination chain
export interface TimeoutMessage {
    kind: "TimeoutMessage";
    // encoded call for HandlerV1.handlePostRequestTimeouts
    calldata: HexString;
}

// This event is emitted on hyperbridge
export interface DestinationFinalizedWithMetadata {
    kind: "DestinationFinalized";
    // Block height of the destination chain that was finalized.
    finalized_height: bigint;
    // The hash of the block where the event was emitted
    block_hash: HexString;
    // The hash of the extrinsic responsible for the event
    transaction_hash: HexString;
    // The block number where the event was emitted
    block_number: bigint;
}

// An error was encountered in the stream, the stream will come to an end.
export interface ErrorWithMetadata {
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
     * Returns the commitment for the provided POST request
     * @param {IPostRequest} post
     * @returns {HexString}
     */
    static post_request_commitment(post: IPostRequest): Promise<HexString>;

    /**
     * Returns the commitment for the provided GET request
     * @param {IPostRequest} get
     * @returns {HexString}
     */
    static get_request_commitment(get: IPostRequest): Promise<HexString>;

    /**
     * Returns the commitment for the provided POST response
     * @param {IPostResponse} response
     * @returns {HexString}
     */
    static post_response_commitment(
        response: IPostResponse,
    ): Promise<HexString>;

    /**
     * Queries the status of a POST request`
     * @param {IPostRequest} request
     * @returns {Promise<MessageStatusWithMeta>}
     */
    query_post_request_status(
        request: IPostRequest,
    ): Promise<MessageStatusWithMeta>;

    /**
     * Queries the status of a GET request`
     * @param {IGetRequest} request
     * @returns {Promise<any>}
     */
    query_get_request_status(
        request: IGetRequest,
    ): Promise<MessageStatusWithMeta>;

    /**
     * Queries the status of a POST response`
     * @param {IPostResponse} response
     * @returns {Promise<MessageStatusWithMeta>}
     */
    query_post_response_status(
        response: IPostResponse,
    ): Promise<MessageStatusWithMeta>;

    /**
     * Return the status of a post request as a `ReadableStream`. If the stream terminates abruptly,
     * perhaps as a result of some error, it can be resumed given some initial state.
     * @param {IPostRequest} request
     * @param {MessageStatusStreamState} state
     * @returns {Promise<ReadableStream<MessageStatusWithMeta>>}
     */
    post_request_status_stream(
        request: IPostRequest,
        state: MessageStatusStreamState,
    ): Promise<ReadableStream<MessageStatusWithMeta>>;

    /**
     * Return the status of a get request as a `ReadableStream`. If the stream terminates abruptly,
     * perhaps as a result of some error, it can be resumed given some initial state.
     * @param {IGetRequest} request
     * @param {MessageStatusStreamState} state
     * @returns {Promise<ReadableStream<MessageStatusWithMeta>>}
     */
    get_request_status_stream(
        request: IGetRequest,
        state: MessageStatusStreamState,
    ): Promise<ReadableStream<MessageStatusWithMeta>>;

    /**
     * Given a post request that has timed out returns a `ReadableStream` that yields a
     * `TimeoutStatus` This function will not check if the request has timed out, only call it
     * when you receive a `MesssageStatus::TimeOut` from `query_request_status` or
     * `request_status_stream`. The stream ends when once it yields a `TimeoutMessage`
     *
     *  If the stream terminates abruptly, perhaps as a result of some error, it can be resumed given some initial state.
     *
     * @param {IPostRequest} request
     * @param {TimeoutStreamState} state
     * @returns {Promise<ReadableStream<TimeoutStatusWithMeta>>}
     */
    timeout_post_request(
        request: IPostRequest,
        state: TimeoutStreamState,
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
