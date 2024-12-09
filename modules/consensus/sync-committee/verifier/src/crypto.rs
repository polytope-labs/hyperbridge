use alloc::vec::Vec;
use bls::{errors::BLSError, types::G1ProjectivePoint};
use sync_committee_primitives::constants::BlsPublicKey;

pub fn pubkey_to_projective(compressed_key: &BlsPublicKey) -> Result<G1ProjectivePoint, BLSError> {
	let affine_point = bls::pubkey_to_point(&compressed_key.to_vec())?;
	Ok(affine_point.into())
}

pub fn subtract_points_from_aggregate(
	aggregate: &BlsPublicKey,
	points: &[BlsPublicKey],
) -> Result<G1ProjectivePoint, BLSError> {
	let aggregate = pubkey_to_projective(aggregate)?;
	let points = points
		.iter()
		.map(|point| pubkey_to_projective(point))
		.collect::<Result<Vec<_>, _>>()?;
	let subset_aggregate = points.into_iter().fold(aggregate, |acc, point| acc - point);
	Ok(subset_aggregate)
}

#[cfg(test)]
mod tests {
	use ark_ec::CurveGroup;
	use sync_committee_primitives::constants::BlsPublicKey;

	use crate::crypto::subtract_points_from_aggregate;
	use hex_literal::hex;

	#[test]
	fn test_signature_verification() {
		let pks = vec![
			hex::decode("882417eb57b98c7dd8e4adb5d4c7b59cb46ad093072f10db99e02597e3432fe094e2698df4c3bf65ff757ac602182f87").unwrap(),
			hex::decode("8ef016d09c49af41d028fdf6ef04972d11f6931bf57f0922df4e77a52847227c880581eebb6b485af1d68bb4895cc35c").unwrap(),
			hex::decode("88b92def24f441be1eba41ff76182e0eb224cf06e751df45635db1530bf37765861c82a8f381f81f6ac6a2b3d3d9875b").unwrap(),
			hex::decode("afc92546e835a4dbe31e2b3a4e6f44a94466a6f9b5752113b9b828349254582eb7b5b596a32b79fc936a82db8802af0c").unwrap(),
			hex::decode("8391e3a00add4bcbe4c339fa7c35238855861cbbc89ceefa6832de6b28bc378a0d038a329636d53404e0deaa444bdfd0").unwrap(),
			hex::decode("9102e77817e572a16fab849f7681d130d10876880d7fe05d40091af93592150ad4829145a7327d125e71a8847a368121").unwrap(),
			hex::decode("8d966a5cfd601661bfb6e15b8c849d3bd85006aec628b44e88022b01054be5159de73f16504a969d6009a59d9214b043").unwrap(),
			hex::decode("b6778f88f9df6d5d09baf9bccd2ea1e4cb88469239a0a14ffcca37fc1c29bad69711dc64fc4e1bb1be0792b005a1729a").unwrap(),
			hex::decode("afc664d1160d2a55fab55fe9d94551b18aa2543f218b9fbdd733509463416c96ee13da6cf75f97165922ca61372c6fb7").unwrap(),
			hex::decode("ad413282bc501315d2cccf8e2a5dd54a5baca851515a04e5f252c98cfeeb670604fa48c707127017e0b8cda218d98207").unwrap()
		];

		let message =
			hex::decode("813a89a296973e35545cfa74fe3efd172a7d19443c97c625d699e9737229b0a2")
				.unwrap();
		let aggregate_signature = hex::decode("a1abfcf9bd54b7a003e1f45f7543b194d8d25b816577b02ee4f1c99aa9821c620be6ecedbc8c5fab64d343a6cc832040029040e591fa24db54f5441f28d73918775e8feeac6177c9e016d2576b982d1cce453896a8aace2bda7374e5a76ce213").unwrap();
		let aggregate_pub_key: BlsPublicKey = hex!("a3f2da752bd1dfc7288b46cc061668856e0cefa93ba6e8ff4699f355138f63a541fdb3444ddebcdce695d6313fa4b244").to_vec().try_into().unwrap();

		let bit_vector = hex::decode("01000100010001000100").unwrap();

		let non_participants = pks
			.into_iter()
			.zip(bit_vector)
			.filter_map(|(pk, bit)| if bit == 0 { Some(pk.try_into().unwrap()) } else { None })
			.collect::<Vec<BlsPublicKey>>();

		let aggregate =
			subtract_points_from_aggregate(&aggregate_pub_key, &non_participants).unwrap();

		let verify = bls::verify(
			&bls::point_to_pubkey(aggregate.into_affine()),
			&message,
			&aggregate_signature,
			&bls::DST_ETHEREUM.as_bytes().to_vec(),
		);

		assert!(verify, "Signature verification failed");
	}
}
