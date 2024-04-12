import { ContractTransactionResponse, Signer } from "ethers";
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
export async function teleport(signer: Signer,transportParam: TeleportParams ) : Promise<ContractTransactionResponse> {
    let response = await gatewayContract(signer).teleport(
        transportParam
    );

    return response;
}


