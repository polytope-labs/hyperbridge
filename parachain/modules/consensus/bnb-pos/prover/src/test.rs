use std::time::Duration;

use bnb_pos_verifier::{primitives::{compute_epoch, Header, EPOCH_LENGTH}, ValidatorData, verify_bnb_header};
use ethers::providers::{Provider, Ws};
use ismp::util::Keccak256;
use sp_core::H160;
use tokio::time::interval;


use crate::BnbPosProver;

pub struct Host;

impl Keccak256 for Host {
    fn keccak256(bytes: &[u8]) -> primitive_types::H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }
}

async fn setup_prover() -> BnbPosProver {
    dotenv::dotenv().ok();
    let consensus_url = std::env::var("BNB_RPC").unwrap();
    let provider = Provider::<Ws>::connect_with_reconnects(consensus_url, 1000).await.unwrap();

    BnbPosProver::new(provider)
}

//#[tokio::test]
async fn verify_bnb_pos_headers() {
    let prover = setup_prover().await;

    let mut interval = interval(Duration::from_secs(2));

    let initial_header = prover.latest_header().await.unwrap();
    let initial_header_epoch = compute_epoch(initial_header.number.low_u64());

    //let initial_header = prover.fetch_header(34786000u64).await.unwrap();
    //let initial_header_epoch = compute_epoch(34786000u64);

    println!(
        "Initial header {:?} in epoch {:?}",
        initial_header,
        initial_header_epoch
    );

    let validator_change_block_number = initial_header_epoch * EPOCH_LENGTH;
    let validator_change_header = prover.fetch_header(validator_change_block_number).await.unwrap();

    println!(
        "Validator header {:?} in epoch {:?}",
        validator_change_header,
        validator_change_block_number
    );

    let (_, validators_data) = prover
        .fetch_proofs_and_validators::<Host>(validator_change_header)
        .await
        .unwrap();
    let mut validators = validators_data.unwrap();

    loop {
        interval.tick().await;
        let latest_header = prover.latest_header().await.unwrap();
        let latest_header_epoch = compute_epoch(latest_header.number.low_u64());

        if initial_header_epoch == latest_header_epoch {
            verify_bnb_header::<Host>(&validators, latest_header.clone()).unwrap();
            println!(
                "Successfully verified header {:?} in epoch {:?}",
                latest_header.number.low_u64(),
                latest_header_epoch
            );
        }

        if initial_header_epoch > latest_header_epoch {
            for epoch in (initial_header_epoch.clone() + 1)..latest_header_epoch {
                let last_header_number_epoch = (epoch * EPOCH_LENGTH) + EPOCH_LENGTH - 1;
                let last_header_in_epoch =
                    prover.fetch_header(last_header_number_epoch.clone()).await.unwrap();

                let previous_epoch = last_header_number_epoch - EPOCH_LENGTH;
                let previous_epoch_header =
                    prover.fetch_header(previous_epoch.clone()).await.unwrap();
                let (previous_epoch_bnb_proof, _previous_epoch_validators_data) = prover
                    .fetch_proofs_and_validators::<Host>(previous_epoch_header)
                    .await
                    .unwrap();
                let previous_epoch_validator_set_size = previous_epoch_bnb_proof.validator_set_size;

                let validator_set_change_epoch: u64 =
                    previous_epoch_validator_set_size as u64 / 2 + previous_epoch;

                if epoch == validator_set_change_epoch {
                    let validator_set_change_epoch_header =
                        prover.fetch_header(epoch.clone()).await.unwrap();
                    let (_, validator_set_change_validators_data) = prover
                        .fetch_proofs_and_validators::<Host>(validator_set_change_epoch_header)
                        .await
                        .unwrap();

                    validators = validator_set_change_validators_data.unwrap();
                }

                verify_bnb_header::<Host>(&validators, last_header_in_epoch.clone()).unwrap();

                println!(
                    "Successfully verified header {:?} in new epoch {:?}",
                    latest_header.number.low_u64(),
                    last_header_number_epoch
                );
            }
        }
    }
}

#[tokio::test]
async fn verify_manually() {
    let mut validators_string = vec![];
    validators_string.push(("0x2465176c461afb316ebc773c61faee85a6515daa", "0x8a923564c6ffd37fb2fe9f118ef88092e8762c7addb526ab7eb1e772baef85181f892c731be0c1891a50e6b06262c816", true));
    validators_string.push(("0x2d4c407bbe49438ed859fe965b140dcf1aab71a9", "0x93c1f7f6929d1fe2a17b4e14614ef9fc5bdc713d6631d675403fbeefac55611bf612700b1b65f4744861b80b0f7d6ab0", true));
    validators_string.push(("0x3f349bbafec1551819b8be1efea2fc46ca749aa1", "0x84248a459464eec1a21e7fc7b71a053d9644e9bb8da4853b8f872cd7c1d6b324bf1922829830646ceadfb658d3de009a", true));
    validators_string.push(("0x61dd481a114a2e761c554b641742c973867899d3", "0x8a80967d39e406a0a9642d41e9007a27fc1150a267d143a9f786cd2b5eecbdcc4036273705225b956d5e2f8f5eb95d25", true));
    validators_string.push(("0x685b1ded8013785d6623cc18d214320b6bb64759", "0x8a60f82a7bcf74b4cb053b9bfe83d0ed02a84ebb10865dfdd8e26e7535c43a1cccd268e860f502216b379dfc9971d358", true));
    validators_string.push(("0x70f657164e5b75689b64b7fd1fa275f334f28e18", "0x96a26afa1295da81418593bd12814463d9f6e45c36a0e47eb4cd3e5b6af29c41e2a3a5636430155a466e216585af3ba7", true));
    validators_string.push(("0x72b61c6014342d914470ec7ac2975be345796c2b", "0x81db0422a5fd08e40db1fc2368d2245e4b18b1d0b85c921aaaafd2e341760e29fc613edd39f71254614e2055c3287a51", true));
    validators_string.push(("0x7ae2f5b9e386cd1b50a4550696d957cb4900f03a", "0xb84f83ff2df44193496793b847f64e9d6db1b3953682bb95edd096eb1e69bbd357c200992ca78050d0cbe180cfaa018e", true));
    validators_string.push(("0x8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec73", "0xa8a257074e82b881cfa06ef3eb4efeca060c2531359abd0eab8af1e3edfa2025fca464ac9c3fd123f6c24a0d78869485", true));
    validators_string.push(("0xa6f79b60359f141df90a0c745125b131caaffd12", "0xb772e180fbf38a051c97dabc8aaa0126a233a9e828cdafcc7422c4bb1f4030a56ba364c54103f26bad91508b5220b741", true));
    validators_string.push(("0xb218c5d6af1f979ac42bc68d98a5a0d796c6ab01", "0xb659ad0fbd9f515893fdd740b29ba0772dbde9b4635921dd91bd2963a0fc855e31f6338f45b211c4e9dedb7f2eb09de7", true));
    validators_string.push(("0xb4dd66d7c2c7e57f628210187192fb89d4b99dd4", "0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", false));
    validators_string.push(("0xbe807dddb074639cd9fa61b47676c064fc50d62c", "0xb1f2c71577def3144fabeb75a8a1c8cb5b51d1d1b4a05eec67988b8685008baa17459ec425dbaebc852f496dc92196cd", true));
    validators_string.push(("0xcc8e6d00c17eb431350c6c50d8b8f05176b90b11","0xb3a3d4feb825ae9702711566df5dbf38e82add4dd1b573b95d2466fa6501ccb81e9d26a352b96150ccbf7b697fd0a419", true));
    validators_string.push(("0xd1d6bf74282782b0b3eb1413c901d6ecf02e8e28", "0x939e8fb41b682372335be8070199ad3e8621d1743bcac4cc9d8f0f6e10f41e56461385c8eb5daac804fe3f2bca6ce739", true));
    validators_string.push(("0xd93dbfb27e027f5e9e6da52b9e1c413ce35adc11", "0xb313f9cba57c63a84edb4079140e6dbd7829e5023c9532fce57e9fe602400a2953f4bf7dab66cca16e97be95d4de7044", true));
    validators_string.push(("0xe2d3a739effcd3a99387d015e260eefac72ebea1", "0x956c470ddff48cb49300200b5f83497f3a3ccb3aeb83c5edd9818569038e61d197184f4aa6939ea5e9911e3e98ac6d21", true));
    validators_string.push(("0xea0a6e3c511bbd10f4519ece37dc24887e11b55d", "0xb2d4c6283c44a1c7bd503aaba7666e9f0c830e0ff016c1c750a5e48757a713d0836b1cabfd5c281b1de3b77d1c192183", true));
    validators_string.push(("0xebe0b55ad7bb78309180cada12427d120fdbcc3a", "0x8fdf49777b22f927d460fa3fcdd7f2ba0cf200634a3dfb5197d7359f2f88aaf496ef8c93a065de0f376d164ff2b6db9a", true));
    validators_string.push(("0xee226379db83cffc681495730c11fdde79ba4c0c", "0xae7bc6faa3f0cc3e6093b633fd7ee4f86970926958d0b7ec80437f936acf212b78f0cd095f4565fff144fd458d233a5b", true));
    validators_string.push(("0xef0274e31810c9df02f98fafde0f841f4e66a1cd", "0x98cbf822e4bc29f1701ac0350a3d042cd0756e9f74822c6481773ceb000641c51b870a996fe0f6a844510b1061f38cd0", true));


    let mut validators_bytes = vec![];

    for (i, (address_string, bls_public_key_string, validator_bit_set)) in validators_string.iter().enumerate() {
        let encoded_address: [u8; 20] = hex::decode(&address_string[2..]).unwrap().as_slice().try_into().unwrap();
        let encoded_bls_public_key: [u8; 48] = hex::decode(&bls_public_key_string[2..]).unwrap().as_slice().try_into().unwrap();

        validators_bytes.push((encoded_address, encoded_bls_public_key, validator_bit_set));

        println!(
            "Validator {}: Encoded Address: {:?}, Encoded BLS Public Key: {:?}",
            i + 1,
            encoded_address,
            encoded_bls_public_key
        );
    }

    validators_bytes.retain(|validator| *validator.2 && (validator.1 != [0; 48]));

    let mut validator_data_vec: Vec<ValidatorData>  = vec![];
    for (i, (address, bls_public_key_string, validator_bit_set)) in validators_bytes.iter().enumerate() {
        println!(
            "Validator {}: Encoded Address: {:?}, Encoded BLS Public Key: {:?}",
            i + 1,
            address,
            bls_public_key_string
        );

        let address_deref = *address;
        let validator_data = ValidatorData {
            address: address_deref.into(),
            bls_public_key: *bls_public_key_string
        };

        validator_data_vec.push(validator_data)

    }

    let prover = setup_prover().await;
    let header =
        prover.fetch_header(34801850).await.unwrap();


    verify_bnb_header::<Host>(&validator_data_vec, header.clone()).unwrap();

}
