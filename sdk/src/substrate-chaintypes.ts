import { URL } from 'url';
// @ts-ignore
global.URL = URL;

const definitions = {
 gargantua: {
  hasher: 'keccakAsU8a',
 },

 polkadot: {
  hasher: 'blake2AsU8a',
 },

 kusama: {
  hasher: 'blake2AsU8a',
 },
};

export default {
 typesBundle: {
  spec: definitions,
 },
};
