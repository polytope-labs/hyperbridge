import { BytesLike, ContractTransactionResponse, Signer } from "ethers";
import { gatewayContract } from "./contracts";
import { TeleportParams } from "./types";
export * from './constants';
export * from './types';



/**
 * 
 * @param signer this an injected signer, could be from metamask or ethers provider
 * @param transportParam this is the teleport params
 * @returns returns the transaction response
 */
export async function teleport(signer: Signer,transportParam: TeleportParams, isTestnet: boolean) : Promise<ContractTransactionResponse> {
    let response = await gatewayContract(signer, isTestnet).teleport(
        transportParam
    );
    
    return response;
}


export async function estimateFee(perByteFee: number, _relayerFee: number, data: Uint8Array ) {
    let fee = (perByteFee * data.length + 1 ) + _relayerFee;
    return fee;
}
