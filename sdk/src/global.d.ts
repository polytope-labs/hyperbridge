import { Store } from '@subql/types-core';
import { Provider, Signer } from 'ethers';
import { Logger } from '@subql/types';

declare global {
 const store: Store;
 const api: Provider | Signer;
 const logger: Logger;
 const chainId: string;
}

export {};