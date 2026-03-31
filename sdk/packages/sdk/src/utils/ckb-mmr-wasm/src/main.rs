use ckb_mmr_wasm::{generate_root_with_proof, verify_proof, MMRResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calldata_bytes: Vec<u8> = (0..128).map(|i| i as u8).collect();
    let tree_size = 100u64;
    let root_with_proof = generate_root_with_proof(&calldata_bytes, tree_size).unwrap();
    let result: MMRResult = serde_json::from_str(&root_with_proof)?;

    println!("Root: {}", result.root);
    println!("Proof: {:?}", result.proof);
    println!("MMR Size: {}", result.mmr_size);
    println!("Leaf Positions: {:?}", result.leaf_positions);

    let proof_result = verify_proof(
        &result.root,
        result.proof,
        result.mmr_size,
        result.leaf_positions[0],
        &calldata_bytes,
    )
    .unwrap();

    println!("Proof Result: {}", proof_result);

    Ok(())
}
