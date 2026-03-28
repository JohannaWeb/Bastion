use reqwest::Client;
use serde_json::Value;
use anyhow::Result;
use k256::ecdsa::{SigningKey, signature::Signer};
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::Utc;
use http::header::{HeaderMap, HeaderValue, HOST, DATE};

pub struct Relay {
    client: Client,
}

impl Relay {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Sends a translated ActivityPub activity to a target inbox with an HTTP Signature.
    pub async fn send_activitypub(
        &self,
        inbox_url: &str,
        activity: Value,
        signing_key_pem: &str,
        key_id: &str,
    ) -> Result<()> {
        let body = serde_json::to_string(&activity)?;
        let host = url::Url::parse(inbox_url)?
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid inbox URL"))?
            .to_string();
        
        let date = Utc::now().to_rfc2822();
        
        // Generate HTTP Signature
        let signing_key = SigningKey::from_pkcs8_pem(signing_key_pem)?;
        let string_to_sign = format!("(request-target): post {}\nhost: {}\ndate: {}", 
            url::Url::parse(inbox_url)?.path(), host, date);
            
        let signature: k256::ecdsa::Signature = signing_key.sign(string_to_sign.as_bytes());
        let signature_b64 = Base64UrlUnpadded::encode_string(&signature.to_bytes());
        
        let signature_header = format!(
            "keyId=\"{}\",algorithm=\"hs2019\",headers=\"(request-target) host date\",signature=\"{}\"",
            key_id, signature_b64
        );

        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(&host)?);
        headers.insert(DATE, HeaderValue::from_str(&date)?);
        headers.insert("Signature", HeaderValue::from_str(&signature_header)?);
        headers.insert("Content-Type", HeaderValue::from_static("application/activity+json"));

        let res = self.client.post(inbox_url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("Failed to send activity: {}", res.status());
        }

        Ok(())
    }

    /// Posts a message to an AT Protocol PDS.
    pub async fn send_atproto(&self, pds_url: &str, record: Value, jwt: &str) -> Result<()> {
        let res = self.client.post(format!("{}/xrpc/com.atproto.repo.createRecord", pds_url))
            .bearer_auth(jwt)
            .json(&record)
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("Failed to post to AT Protocol: {}", res.status());
        }

        Ok(())
    }
}
