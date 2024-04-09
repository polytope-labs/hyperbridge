// global API exportation is done here 

import { ethers } from "ethers";
import { PROVIDER } from "./constants";
import { gateway } from "./contracts";
import { TeleportParams } from "./types";
export * from './constants';
export * from './types';



export async function teleport (
    transportParam: TeleportParams,
    privateKey: string
) {
    let wallet = new ethers.Wallet(privateKey, PROVIDER);
    let hyperbridgeGateway = gateway(wallet);

    let get_synchro_NonFungible = await hyperbridgeGateway.teleport(transportParam);
    return get_synchro_NonFungible
}