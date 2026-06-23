// PrioritizeVeto encodes to 0 bytes (PhantomData); CheckMetadataHash adds a 1-byte mode field.
// Without these entries polkadot.js miscounts extension bytes and reads a garbage call index.
const signedExtensions = {
	PrioritizeVeto: { extrinsic: {}, payload: {} },
	CheckMetadataHash: { extrinsic: { mode: "u8" }, payload: {} },
}

export default {
	typesBundle: {
		spec: {
			gargantua: {
				hasher: "keccakAsU8a",
				signedExtensions,
			},
			nexus: {
				hasher: "keccakAsU8a",
				signedExtensions,
			},
		},
	},
}
