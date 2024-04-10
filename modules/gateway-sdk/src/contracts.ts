import { Provider, Signer } from "ethers";
import { GATEWAY } from "./constants";
import { TokenGateway__factory } from "./types/factories/contracts/TokenGateway__factory";


export function gateway(signer: Signer| Provider) {
    return TokenGateway__factory.connect(GATEWAY, signer);
} 

