extern crate alloc;
use alloc::string::String;

/// Lowercase-hex encoding of a 32-byte commitment hash. Inlined to avoid
/// pulling a heavier hex crate just for log surface.
pub fn hex32(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}
