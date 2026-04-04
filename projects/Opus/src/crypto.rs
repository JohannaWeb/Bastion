use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DEMO_LEDGER_SECRET: &str = "opus-demo-ledger-secret";

pub fn sign(signer_did: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(signing_key().as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(signer_did.as_bytes());
    mac.update(b":");
    mac.update(payload.as_bytes());
    format!("mac:{}", hex_encode(&mac.finalize().into_bytes()))
}

pub fn verify(signer_did: &str, payload: &str, signature: &str) -> bool {
    sign(signer_did, payload) == signature
}

fn signing_key() -> String {
    std::env::var("OPUS_LEDGER_SECRET").unwrap_or_else(|_| DEMO_LEDGER_SECRET.to_string())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}
