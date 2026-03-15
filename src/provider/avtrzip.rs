use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;

use crate::{
    provider::{Provider, ProviderKind},
    USER_AGENT,
};

const URL: &str = "http://api.avtr.zip/v1/avatars/push";

#[derive(Default)]
pub struct AvtrZip {
    client: Client,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AvtrZipResponse {
    Success {
        #[allow(dead_code)]
        success: bool,
        #[serde(rename = "isNew")]
        is_new:  bool,
    },
    Error {
        error: String,
    },
}

#[async_trait]
impl Provider for AvtrZip {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AVTRZIP
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let json = json!({ "avatarId": avatar_id });

        debug!("[{kind}] Sending {json:#?}");

        let response = self
            .client
            .post(URL)
            .header("User-Agent", USER_AGENT)
            .json(&json)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{kind}] {status} | {text}");

        match status {
            StatusCode::OK => {
                let data = serde_json::from_str::<AvtrZipResponse>(&text)?;
                match data {
                    AvtrZipResponse::Success { is_new, .. } => Ok(is_new),
                    AvtrZipResponse::Error { error } => bail!("[{kind}] {error}"),
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{kind}] 429 Rate Limit, Please Wait 10 seconds...");
                tokio::time::sleep(Duration::from_secs(10)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await
            }
            _ => bail!("[{kind}] {status} | {text}"),
        }
    }
}
