use serde_json::Value;
use anyhow::{Result, anyhow};

pub struct Resolver;

impl Resolver {
    /// Resolves a DID (PLC or Web) to a DidDocument or relevant metadata.
    /// This is a simplified version for the bridge demo.
    pub async fn resolve(did: &str) -> Result<Value> {
        if did.starts_with("did:plc:") {
            let res = reqwest::get(format!("https://plc.directory/{}", did)).await?;
            if res.status().is_success() {
                return Ok(res.json().await?);
            }
        } else if did.starts_with("did:web:") {
            let host = did.trim_start_matches("did:web:");
            let res = reqwest::get(format!("https://{}/.well-known/did.json", host)).await?;
            if res.status().is_success() {
                return Ok(res.json().await?);
            }
        }
        
        Err(anyhow!("Unsupported or unresolvable DID: {}", did))
    }
}
