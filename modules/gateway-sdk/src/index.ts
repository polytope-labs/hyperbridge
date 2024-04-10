// global API exportation is done here 

import { Provider, Signer, ethers } from "ethers";
import { gateway } from "./contracts";
import { TeleportParams } from "./types";
export * from './constants';
import { AllowanceProvider, PERMIT2_ADDRESS } from '@uniswap/permit2-sdk'
import { MaxAllowanceTransferAmount, PermitSingle } from '@uniswap/permit2-sdk'

export * from './types';



export async function teleport (
    transportParam: TeleportParams,
    signer: Signer

) {
    let hyperbridgeGateway = gateway(signer);

    let teleport = await hyperbridgeGateway.teleport(transportParam);
    return teleport
}

function toDeadline(expiration: number): number {
    return Math.floor((Date.now() + expiration) / 1000)
    }



export async function permit (token: string, user: string, ROUTER_ADDRESS: string,  signer: Provider) {
    
    const allowanceProvider = new AllowanceProvider(signer, PERMIT2_ADDRESS)
    const { amount, nonce, expiration}= await allowanceProvider.getAllowanceData(user, token, ROUTER_ADDRESS);

    const permitSingle: PermitSingle = {
        details: {
        token,
        amount: MaxAllowanceTransferAmount,
       
        expiration: toDeadline( 1000 * 60 * 60 * 24 * 30),
        nonce,
        },
        spender: user,
        sigDeadline: toDeadline( 1000 * 60 * 60 * 30),
        }

        return permitSingle
}