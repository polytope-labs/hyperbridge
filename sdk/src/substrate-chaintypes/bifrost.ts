import { typesBundleForPolkadot } from '@bifrost-finance/type-definitions';

export default {
 typesBundle: {
  spec: {
   bifrost: typesBundleForPolkadot,
  },
 },
 types: {
  DispatchError: 'DispatchErrorPre6First',
 },
};
