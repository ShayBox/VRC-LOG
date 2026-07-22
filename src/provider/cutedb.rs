use std::time::Duration;

use anyhow::{bail, Result};
use flume::{Receiver, Sender};
use reqwest::{Client, StatusCode};
use serde_json::json;
use tokio::time::Instant;

use crate::{
    provider::{Provider, ProviderKind},
    USER_AGENT,
};

const URL: &str = "https://avtr.icu/upload-bulk";

const FLUSH_INTERVAL: Duration = Duration::from_secs(2 * 60);
const FLUSH_THRESHOLD: usize = 100;

const RETRY_LIMIT: usize = 5;

const LOG_NAME: &str = "CuteDB";

pub struct CuteDB {
    sender: Sender<String>,
}

pub struct CuteDBActor {
    client:         Client,
    buffer:         Vec<String>,
    channel:        Receiver<String>,
    flush_interval: Duration,
    last_flush:     Instant,
}

impl CuteDBActor {
    #[must_use]
    pub fn new() -> (Self, Sender<String>) {
        let (tx, rx) = flume::bounded(FLUSH_THRESHOLD);

        (
            Self {
                client:         Client::default(),
                buffer:         Vec::new(),
                channel:        rx,
                flush_interval: FLUSH_INTERVAL,
                last_flush:     Instant::now(),
            },
            tx,
        )
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Ok(id) = self.channel.recv_async().await {
            self.buffer.push(id);

            if self.buffer.len() >= FLUSH_THRESHOLD
                || self.last_flush.elapsed() > self.flush_interval
            {
                match self.flush_buffer().await {
                    Ok(()) => (),
                    Err(err) => error!("[{LOG_NAME}]: Failed to flush buffer: {err}"),
                }
            }
        }

        if !self.buffer.is_empty()
            && let Err(err) = self.flush_buffer().await
        {
            error!("[{LOG_NAME}]: Failed to flush buffer on shutdown: {err}");
        }

        Ok(())
    }

    pub async fn flush_buffer(&mut self) -> Result<()> {
        let json: Vec<_> = self
            .buffer
            .iter()
            .map(|id| json!({ "id": id }))
            .collect();

        let mut current_try = 0;
        let mut success = false;
        while current_try < RETRY_LIMIT && !success {
            if current_try != 0 {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            debug!("[{LOG_NAME}] (try {current_try}) Sending {json:#?}");
            current_try += 1;

            let response = self
                .client
                .post(URL)
                .header("User-Agent", USER_AGENT)
                .json(&json)
                .timeout(Duration::from_secs(5))
                .send()
                .await?;

            let status = response.status();
            let text = response.text().await?;
            debug!("[{LOG_NAME}] {status} | {text}");

            success = match status {
                StatusCode::OK => true,
                StatusCode::TOO_MANY_REQUESTS => {
                    warn!("[{LOG_NAME}] 429 Rate Limit, trying again in 10 seconds");
                    false
                }
                _ => {
                    error!("[{LOG_NAME}] Unknown Error: {status} | {text}");
                    false
                }
            };
        }

        if current_try >= RETRY_LIMIT {
            bail!("[{LOG_NAME}] Failed after {current_try} retries to flush buffer, aborting");
        }

        self.buffer.clear();
        self.last_flush = Instant::now();

        Ok(())
    }
}

impl CuteDB {
    #[must_use]
    pub const fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }
}

#[async_trait::async_trait]
impl Provider for CuteDB {
    fn kind(&self) -> ProviderKind {
        ProviderKind::CUTEDB
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        self.sender.send_async(avatar_id.to_string()).await?;
        Ok(true)
    }
}
