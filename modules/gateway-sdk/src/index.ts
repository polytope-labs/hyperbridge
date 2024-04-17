import { AbiCoder, ContractTransactionResponse, Provider, Signer, ethers } from "ethers";
import { erc20Contract, gatewayContract } from "./contracts";
import { TeleportParams } from "./types";
export * from './constants';
export * from './types';
import {HyperClient, IPostRequest, MessageStatus, MessageStatusWithMeta, TimeoutStatusWithMeta} from "@polytope-labs/hyperclient";



/**
 *
 * @param {Signer} signer this an injected signer, could be from metamask or ethers provider
 * @param {TeleportParams} transportParam this is the teleport params
 * @returns {Promise<ContractTransactionResponse>} returns the transaction response
 */
export async function teleport(signer: Signer,transportParam: TeleportParams, isTestnet: boolean) : Promise<ContractTransactionResponse> {
    let response = await gatewayContract(signer, isTestnet).teleport(
        transportParam
    );

    return response;
}


/**
 *
 * @description This function estimates the fee for teleporting
 * @param {number} perByteFee - this is the current per byte fee
 * @param {number} relayerFee - this is the fee to be paid to the relayer
 * @param {Uint8Array} data - this is the data to be teleported
 * @returns {Promise<number>} function returns the estimated fee
 */
export async function estimateFee(perByteFee: number, relayerFee: number, data: Uint8Array ): Promise<number> {
    let fee = (perByteFee * data.length + 1 ) + relayerFee;
    return fee;
}


/**
 *
 * @description This function checks if a user has enough ERC20 token
 * @param {string} tokenAddress - this is the token address to be checked against
 * @param {string} userAddress - this is the user address that should have the specified amount of token
 * @param {number} targetAmount - this is the target amount of token the user should have
 * @returns function returns true if the user has enough tokens
 */
export async function hasEnoughTokens(tokenAddress: string, userAddress: string, targetAmount: number, provider: Provider): Promise<boolean> {
  const amountInWei = ethers.parseEther(targetAmount.toString());
  const balance = await erc20Contract(provider, tokenAddress).balanceOf(userAddress);
  return balance.gte(amountInWei);
}


/**
 *
 * @description This function checks if a user has enough ERC20 token allowance
 * @param {string} tokenAddress - this is the token address to be checked against
 * @param {string} userAddress - this is the user address that should have the specified amount of token
 * @param {string} spender - this is the address that is allowed to spend the token
 * @param {number} targetAmount - this is the target amount of token the user should have
 * @param {Provider} provider - this is a provider object from ethers.js
 * @returns function returns true if the user has enough tokens
 */
export async function hasEnoughAllowance(tokenAddress: string, userAddress: string, spender: string, targetAmount: number, provider: Provider): Promise<[boolean, BigInt]> {
  const amountInWei = ethers.parseEther(targetAmount.toString());
  const allowance = await erc20Contract(provider, tokenAddress).allowance(userAddress, spender);

   return [allowance.gte(amountInWei), allowance];
}


/**
 * @description This function makes a call to approve a spender to spend a specified amount of token
 * @param {string} tokenAddress - this is the address of the token to be approved
 * @param {Signer} signer - this is the address of the token owner
 * @param {string} spender - this is the address that is allowed to spend the token
 * @param {amount} targetAmount - this is the target amount of token the user should have. Note this is the amount before adding decimals
 * @returns function returns true if the user has enough tokens
 */
export async function handleAllowance(
    signer: Signer,
    tokenAddress: string,
    spender: string,
    targetAmount: number
): Promise<ContractTransactionResponse> {
    const amountInWei = ethers.parseEther(targetAmount.toString());
    const response = await erc20Contract(signer, tokenAddress).approve(spender, amountInWei);
    return response as ContractTransactionResponse;
}


/**
 * @description This function returns the current status of a teleport request
 * @param {HyperClient} client - this is the hyper-client instance
 * @param {IPostRequest} request - this is the teleport request
 * @returns {Promise<MessageStatus>} function returns the current status of the teleport request
 */
export async function getTeleportStatus(client: HyperClient, request: IPostRequest): Promise<MessageStatus> {
  let status = await client.query_request_status(request);
  return status;
}


/**
 * @description This function returns a readable stream of the current status of a teleport request
 * @param {HyperClient} client - this is the hyper-client instance
 * @param {IPostRequest} request - this is the teleport request
 * @returns {Promise<ReadableStream<MessageStatusWithMeta>>} function returns the current status of the teleport request
 */
export async function getTeleportStatusStream(client: HyperClient, request: IPostRequest): Promise<ReadableStream<MessageStatusWithMeta>> {
  let status = await client.request_status_stream(request);
  return status;
}


/**
* @description This function returns the TX data to be used timeout the teleport request
* @param {HyperClient} client - this is the hyper-client instance
* @param {IPostRequest} request - this is the teleport request
* @returns {Promise<ReadableStream<TimeoutStatusWithMeta>>} function returns the TX data
*/
export async function getTeleportTxData(client: HyperClient, request: IPostRequest): Promise<ReadableStream<TimeoutStatusWithMeta>> {
  const stream = await client.timeout_post_request(request);
  return stream;
}


/**
* @description This function is used to encode the teleport calldata used during teleportWithCall
* @param {string} target - this is the contract to be called by the call dispatcher
* @param {string} data - this is the call data for the call
* @returns {string} function returns the TX data
*/
export function encodeTeleportCalldata(target: string, data: string): string {
  let calldata =  AbiCoder.defaultAbiCoder().encode(["tuple(address, bytes)"], [[target, data]]);
  return calldata;
}
