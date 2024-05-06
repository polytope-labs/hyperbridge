use backend_interface::Backend;
use bn254_blackbox_solver::Bn254BlackBoxSolver;
use nargo::{
    artifacts::{debug::DebugArtifact, program::ProgramArtifact},
    ops::DefaultForeignCallExecutor,
};
use noirc_abi::{input_parser::Format, MAIN_RETURN_NAME};
use noirc_driver::CompiledProgram;
use serde::{Deserialize, Serialize};
use sp_core::H160;

/// Params for the prover
#[derive(Deserialize, Serialize)]
pub struct ProverParams {
    /// Vote information
    pub(crate) votes: Vec<Vote>,
    /// Signed message
    pub(crate) msg: [H160; 2],
    /// Mmr Root
    pub(crate) root: [H160; 2],
    /// Sibling authorities
    pub(crate) siblings: Vec<Sibling>,
}

/// A single vote from an authority. We represent 32 bytes using the first (msb) 16 bits of two
/// Field elements;
#[derive(Deserialize, Serialize)]
pub struct Vote {
    /// Public key of the authority
    pub key: PublicKey,
    /// signature data
    pub signature: [H160; 4],
    /// index of the authority in the set
    pub index: H160,
}

/// Secp256k1 public key encoded as Field elements
#[derive(Deserialize, Serialize)]
pub struct PublicKey {
    /// x component of the authority public key
    pub x: [H160; 2],
    /// y component of the authority public key
    pub y: [H160; 2],
}

/// Rather than a sparse merkle tree as proof,
/// we simply reveal the full tree. Much cheaper in noir.
#[derive(Deserialize, Serialize)]
pub struct Sibling {
    /// pre-hashed authority address
    pub hash: [H160; 2],
    /// index of the authority in the set
    pub index: H160,
}

/// Prover instance
pub struct Prover {
    // barretenberg backend
    backend: Backend,
    // compiled program for proving
    program: CompiledProgram,
}

impl Clone for Prover {
    fn clone(&self) -> Self {
        Self {
            program: self.program.clone(),
            backend: Backend::new(self.backend.name().to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
pub enum Network {
    #[display("rococo")]
    Rococo,
    #[display("polkadot")]
    Polkadot,
    #[display("kusama")]
    Kusama,
}

impl Prover {
    pub fn new(network: Network) -> Result<Self, anyhow::Error> {
        let backend = Backend::new("acvm-backend-barretenberg".into());
        let (program_artifact, mut debug_artifact) = artifacts(network)?;

        let program = CompiledProgram {
            hash: program_artifact.hash,
            circuit: program_artifact.bytecode,
            abi: program_artifact.abi,
            noir_version: program_artifact.noir_version,
            debug: debug_artifact.debug_symbols.remove(0),
            file_map: debug_artifact.file_map,
            warnings: debug_artifact.warnings,
        };

        let prover = Self { backend, program };

        Ok(prover)
    }

    /// Generate a Plonk proof for the given inputs
    pub fn prove(&self, params: ProverParams) -> Result<Vec<u8>, anyhow::Error> {
        let blackbox_solver = Bn254BlackBoxSolver::new();
        // generate witness
        let json = serde_json::to_string(&params)?;
        let mut inputs_map = Format::Json.parse(&json, &self.program.abi)?;
        let _ = inputs_map.remove(MAIN_RETURN_NAME);
        let initial_witness = self.program.abi.encode(&inputs_map, None)?;

        let witness = nargo::ops::execute_circuit(
            &self.program.circuit,
            initial_witness,
            &blackbox_solver,
            &mut DefaultForeignCallExecutor::new(true, None),
        )?;

        let proof = self.backend.prove(&self.program.circuit, witness, false)?;

        Ok(proof)
    }
}

/// Return the appropriate artifact for given chain
fn artifacts(network: Network) -> Result<(ProgramArtifact, DebugArtifact), anyhow::Error> {
    let (program_artifact, debug_artifact) = match network {
        Network::Polkadot => {
            let program_artifact: ProgramArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/polkadot.json"))?;
            let debug_artifact: DebugArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/debug_polkadot.json"))?;

            (program_artifact, debug_artifact)
        }
        Network::Kusama => {
            let program_artifact: ProgramArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/kusama.json"))?;
            let debug_artifact: DebugArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/debug_kusama.json"))?;
            (program_artifact, debug_artifact)
        }
        Network::Rococo => {
            let program_artifact: ProgramArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/rococo.json"))?;
            let debug_artifact: DebugArtifact =
                serde_json::from_slice(include_bytes!("../artifacts/debug_rococo.json"))?;
            (program_artifact, debug_artifact)
        }
    };

    Ok((program_artifact, debug_artifact))
}

/// Convert to a field element
pub fn to_field_element(input: &[u8; 32]) -> [H160; 2] {
    let (hi, lo) = input.split_at(16);

    [to_h160(hi), to_h160(lo)]
}

fn to_h160(slice: &[u8]) -> H160 {
    let mut buf = [0u8; 20];

    buf[4..].copy_from_slice(slice);

    H160::from(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_merkle::{Hasher, MerkleTree};
    use sp_core::{ecdsa, keccak_256, Pair, H160, H256};

    #[derive(Clone)]
    pub struct Keccak;

    impl Hasher for Keccak {
        type Hash = [u8; 32];

        fn hash(data: &[u8]) -> [u8; 32] {
            keccak_256(data)
        }
    }

    #[test]
    fn generate_ecdsa() -> Result<(), anyhow::Error> {
        let message = H256::random();

        let stuff = vec![
            (1000usize, 667usize, 333usize, "kusama"),
            (110, 74, 36, "rococo"),
            (297, 199, 98, "polkadot"),
        ];

        for (total, major, _minor, name) in stuff {
            let mut keys_x = vec![];
            let mut keys_y = vec![];
            let mut signatures = vec![];
            let mut leaves = vec![];

            for _ in 0..total {
                let (pair, _) = ecdsa::Pair::generate();
                let signature = pair.sign_prehashed(&message.0);
                let public = libsecp256k1::PublicKey::parse_compressed(&pair.public().0)
                    .unwrap()
                    .serialize();
                let (x, y) = public[1..].split_at(32);

                keys_x.push(x.to_vec());
                keys_y.push(y.to_vec());
                signatures.push(signature.0[..64].to_vec());
                let leaf = H160::from_slice(&keccak_256(&public[1..])[12..]);
                leaves.push(leaf);
            }

            let keys = keys_x
                .into_iter()
                .zip(keys_y.into_iter())
                .take(major)
                .map(|(x, y)| {
                    let x = {
                        let (hi, lo) = x.split_at(16);
                        [to_h160(hi), to_h160(lo)]
                    };

                    let y = {
                        let (hi, lo) = y.split_at(16);
                        [to_h160(hi), to_h160(lo)]
                    };

                    PublicKey { x, y }
                })
                .collect::<Vec<_>>();

            let leaf_hashes = leaves
                .iter()
                .map(|l| keccak_256(l.as_bytes()))
                .collect::<Vec<_>>();
            let tree = MerkleTree::<Keccak>::from_leaves(&leaf_hashes);
            let root = H256(tree.root().unwrap());

            let (leaves, siblings) = leaves.split_at(major);

            let siblings = siblings
                .iter()
                .enumerate()
                .map(|(i, address)| {
                    let hash = keccak_256(address.as_bytes());
                    let (hi, lo) = hash.split_at(16);
                    Sibling {
                        hash: [to_h160(hi), to_h160(lo)],
                        index: H160::from_low_u64_be((major + i) as u64),
                    }
                })
                .collect();

            assert_eq!(leaves.len(), major);

            let votes = signatures
                .into_iter()
                .zip(keys)
                .enumerate()
                .take(major)
                .map(|(i, (sig, key))| {
                    let (left, right) = sig.split_at(32);
                    let (a1, a2) = left.split_at(16);
                    let (a3, a4) = right.split_at(16);

                    let signature = [to_h160(a1), to_h160(a2), to_h160(a3), to_h160(a4)];

                    Vote {
                        key,
                        signature,
                        index: H160::from_low_u64_be(i as u64),
                    }
                })
                .collect();

            let (hi, lo) = message.0.split_at(16);
            let msg = [to_h160(hi), to_h160(lo)];

            let (hi, lo) = root.0.split_at(16);
            let root = [to_h160(hi), to_h160(lo)];

            let params = ProverParams {
                votes,
                siblings,
                msg,
                root,
            };
            std::fs::write(
                format!("./{name}/Prover.toml"),
                toml::to_string_pretty(&params).unwrap(),
            )
            .unwrap();

            // let mut prover = Prover::new()?;
            //
            // let bytes = prover.prove(params)?;
            // println!("{}", bytes::to_hex(&bytes, false));
        }

        Ok(())
    }
}
