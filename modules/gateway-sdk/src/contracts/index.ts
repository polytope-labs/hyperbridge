import { Signer ,Provider, ethers} from "ethers";
import { GATEWAY } from "../constants";
import abi from "./gateway";




export function gatewayContract(signer: Signer| Provider) {
   return new ethers.Contract(GATEWAY, abi, signer);
}

