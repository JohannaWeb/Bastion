pub fn sign(signer_did: &str, payload: &str) -> String {
    let material = format!("{signer_did}:{payload}");
    format!("sig:{:016x}", fnv1a64(material.as_bytes()))
}

pub fn verify(signer_did: &str, payload: &str, signature: &str) -> bool {
    sign(signer_did, payload) == signature
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
