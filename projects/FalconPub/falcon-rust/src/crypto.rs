use k256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use base64ct::{Base64UrlUnpadded, Encoding};
use rand_core::OsRng;
use anyhow::{anyhow, Result};

/// A secp256k1 (ES256K) key pair.
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a fresh random key pair.
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::random(&mut OsRng),
        }
    }

    /// Load from raw 32-byte secret scalar.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            signing_key: SigningKey::from_bytes(bytes.into())?,
        })
    }

    /// Sign `message` and return a URL-safe Base64 (no padding) signature.
    pub fn sign(&self, message: &[u8]) -> String {
        let sig: Signature = self.signing_key.sign(message);
        Base64UrlUnpadded::encode_string(&sig.to_bytes())
    }

    /// Verify a signature produced by `sign`.
    pub fn verify(public_key_bytes: &[u8], message: &[u8], signature_b64: &str) -> Result<()> {
        let sig_bytes = Base64UrlUnpadded::decode_vec(signature_b64)
            .map_err(|e| anyhow!("base64 decode: {e}"))?;
        let sig = Signature::from_slice(&sig_bytes)
            .map_err(|e| anyhow!("bad signature: {e}"))?;
        let vk = VerifyingKey::from_sec1_bytes(public_key_bytes)
            .map_err(|e| anyhow!("bad public key: {e}"))?;
        vk.verify(message, &sig)
            .map_err(|_| anyhow!("signature verification failed"))
    }

    /// Compressed SEC1-encoded public key bytes.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_sign_verify() {
        let kp = KeyPair::generate();
        let msg = b"hello falcon activitypub";
        let sig = kp.sign(msg);
        let pub_key = kp.public_key_bytes();
        KeyPair::verify(&pub_key, msg, &sig).expect("verification should succeed");
    }

    #[test]
    fn wrong_message_fails() {
        let kp = KeyPair::generate();
        let sig = kp.sign(b"correct message");
        let pub_key = kp.public_key_bytes();
        assert!(KeyPair::verify(&pub_key, b"wrong message", &sig).is_err());
    }
}
