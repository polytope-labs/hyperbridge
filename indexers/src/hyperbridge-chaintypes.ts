import type { OverrideBundleDefinition } from "@polkadot/types/types";

import { keccakAsU8a } from "@polkadot/util-crypto";

const definitions: OverrideBundleDefinition = {
  hasher: keccakAsU8a,
};

export default { typesBundle: { spec: { hyperbridge: definitions } } };
