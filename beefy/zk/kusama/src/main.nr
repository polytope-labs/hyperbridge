use dep::std;
use dep::std::collections::vec::Vec;


/// A single vote from an authority. We represent 32 bytes using the first (msb) 16 bits of two Field elements;
struct Vote {
    key: PublicKey,
    /// signature data
    signature: [Field; 4],
    /// index of the authority in the set
    index: Field
}

/// Secp256k1 public key encoded as Field elements
struct PublicKey {
    /// x component of the authority public key
    x: [Field; 2],
    /// y component of the authority public key
    y: [Field; 2],
}

/// Rather than a sparse merkle tree as proof,
/// we simply reveal the full tree. Much cheaper in noir.
struct Sibling {
    /// pre-hashed authority address
    hash: [Field; 2],
    /// index of the authority in the set
    index: Field
}

/// An authority
struct Authority {
    /// hashed address
    address: [u8; 32],
    /// index of the authority in the set
    index: Field,
}

/// Produce a hash of two bytes32
pub fn hash2(first: [u8; 32], second: [u8; 32]) -> [u8; 32] {
    let mut slice: [u8] = [0; 64];

    for i in 0..32 {
        slice[i] = first[i];
        slice[i + 32] = second[i];
    }

    std::hash::keccak256(slice, 64)
}

/// Produce a byte32 from two field elements by concating their msb
pub fn recover_array32(fields: [Field; 2]) -> [u8; 32] {
    let mut array: [u8; 32] = [0; 32];
    let left = fields[0].to_be_bytes(16);
    let right = fields[1].to_be_bytes(16);

    for i in 0..16 {
        array[i] = left[i];
        array[i + 16] = right[i];
    }

    array
}

/// Recover the ethereum address from the public key and hash it.
pub fn recover_address_hash(x: [u8; 32], y: [u8; 32]) -> [u8; 32] {
    let hash = hash2(x, y);
    let mut address: [u8] = [0; 20];

    for i in 0..20 {
        address[i] = hash[i + 12];
    }

    std::hash::keccak256(address, 20)
}

/// Produce a byte64 from four field elements by concating their msb
pub fn recover_array64(fields: [Field; 4]) -> [u8; 64] {
    let mut array: [u8; 64] = [0; 64];
    let a1 = fields[0].to_be_bytes(16);
    let a2 = fields[1].to_be_bytes(16);
    let a3 = fields[2].to_be_bytes(16);
    let a4 = fields[3].to_be_bytes(16);

    for i in 0..16 {
        array[i] = a1[i];
        array[i + 16] = a2[i];
        array[i + 32] = a3[i];
        array[i + 48] = a4[i];
    }

    array
}

// signature verification
fn verify_votes(combined: &mut [[u8; 32]; 1000], votes: [Vote; 667], msg: [Field; 2]) {
    let msg = recover_array32(msg);

    for i in 0..667 {
        let x = recover_array32(votes[i].key.x);
        let y = recover_array32(votes[i].key.y);
        let sig = recover_array64(votes[i].signature);
        assert(std::ecdsa_secp256k1::verify_signature(x, y, sig, msg));
        combined[votes[i].index] = recover_address_hash(x, y);
    }
}

// membership verification
fn check_membership(combined: &mut [[u8; 32]; 1000], siblings: [Sibling; 333], root: [Field; 2]) {
    for i in 0..333 {
        combined[siblings[i].index] = recover_array32(siblings[i].hash);
    }

    let mut vector = Vec::from_slice(combined.as_slice());

    // pre-set iteration lengths
    let lens = [500, 250, 125, 63, 32, 16, 8, 4, 2, 1];
    for height in 0..10 {
        let mut next: Vec<[u8; 32]> = Vec::new();
        for i in 0..lens[height] {
            let p = vector.len();
            let j = (i * 2);
            if (j + 1) as u16 >= p as u16 {
                next.push(vector.get(j));
            } else {
                next.push(hash2(vector.get(j), vector.get(j + 1)));
            }
        }
        vector = next;
    }

    // sanity check
    assert(vector.len() == 1);
    // check the root
    assert(recover_array32(root) == vector.get(0));
}

fn main(
    siblings: [Sibling; 333],
    votes: [Vote; 667],
    msg: pub [Field; 2],
    root: pub [Field; 2]
) {
    let mut combined: [[u8; 32]; 1000] = [[0; 32]; 1000];
    verify_votes(&mut combined, votes, msg);
    check_membership(&mut combined, siblings, root);
}
