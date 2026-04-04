use reqwest::Url;
use serde_json::Value;

pub struct RemoteDidResolver {
    base_url: String,
}

impl RemoteDidResolver {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub async fn resolve(&self, did: &str) -> Result<Value, String> {
        let mut url = Url::parse(&format!("{}/api/did/resolve", self.base_url))
            .map_err(|e| format!("Invalid resolver base URL: {e}"))?;
        url.query_pairs_mut().append_pair("did", did);

        let resp = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to reach ProjectFalcon: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("ProjectFalcon error: {}", resp.status()));
        }

        let doc: Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid DID document from ProjectFalcon: {e}"))?;

        Ok(doc)
    }
}
