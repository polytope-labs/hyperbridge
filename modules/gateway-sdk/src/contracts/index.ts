import { Signer ,Provider, ethers} from "ethers";
import { GATEWAY_TESTNEST, GATEWAY_MAINNET } from "../constants";
import abi from "./gateway";




export function gatewayContract(signer: Signer| Provider, isTestnet: boolean) {
    if (isTestnet) {
        return new ethers.Contract(GATEWAY_TESTNEST, abi, signer);
    }

    return new ethers.Contract(GATEWAY_MAINNET, abi, signer);
}

