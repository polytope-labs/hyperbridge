import { Signer ,Provider, ethers} from "ethers";
import { GATEWAY_TESTNEST, GATEWAY_MAINNET } from "../constants";
import gatewayAbi from "./gateway";
import erc20Abi from "./erc20";





/**
 *   @description This function returns the gateway contract instance binded with the signer object
 *   @param signer - Signer object from ethers.js
*/
export function gatewayContract(signer: Signer| Provider, isTestnet: boolean) {
    if (isTestnet) {
        return new ethers.Contract(GATEWAY_TESTNEST, gatewayAbi, signer);
    }
    return new ethers.Contract(GATEWAY_MAINNET, gatewayAbi, signer);
}


/**
 *   @description This function returns the erc20 contract instance binded with the signer object
 *   @param signer - Signer object from ethers.js
*/
export function erc20Contract(signer: Signer| Provider, tokenAddress: string) {
    return new ethers.Contract(tokenAddress, erc20Abi, signer);
}
