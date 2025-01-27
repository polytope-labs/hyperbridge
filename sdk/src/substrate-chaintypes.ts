import { typesBundleForPolkadot } from '@bifrost-finance/type-definitions';
import { URL } from 'url';
// @ts-ignore
global.URL = URL;

const definitions = {
 gargantua: {
  hasher: 'keccakAsU8a',
 },
};

export default {
 typesBundle: {
  spec: {
   ...definitions,
   bifrost: typesBundleForPolkadot,
  },
 },
 types: {
  DispatchError: 'DispatchErrorPre6First',
 },
};
