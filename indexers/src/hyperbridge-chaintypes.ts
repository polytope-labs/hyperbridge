import { URL } from "url";
// @ts-ignore
global.URL = URL;

const definitions = {
  hasher: "keccakAsU8a",
};

export default { typesBundle: { spec: { gargantua: definitions } } };
