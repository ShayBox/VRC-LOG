use std::time::Duration;

use anyhow::{Result, bail};
use colored::{Color, Colorize};
use flume::{Receiver, Sender};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use terminal_link::Link;
use tokio::time::Instant;

use crate::{
    USER_AGENT,
    provider::{Provider, ProviderKind},
    settings::Settings,
};

const INGEST_BASE_URL: &str = "https://avtr.fumikoecho.net/api/integrations/vrc-log/";

const WEBSITE_URL: &str = "https://avtr.fumikoecho.net";

const FLUSH_INTERVAL: Duration = Duration::from_mins(2);
const FLUSH_THRESHOLD: usize = 100;

const RETRY_LIMIT: usize = 5;

const LOG_NAME: &str = "KitsuneDB";

pub struct KitsuneDB {
    sender: Sender<String>,
}

pub struct KitsuneDBActor<'s> {
    settings:       &'s Settings,
    client:         Client,
    buffer:         Vec<String>,
    base_url:       String,
    channel:        Receiver<String>,
    flush_interval: Duration,
    last_flush:     Instant,
}

impl<'s> KitsuneDBActor<'s> {
    /// # Errors
    /// Will return `Err` if anything errors
    pub async fn run(&mut self) -> anyhow::Result<()> {
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

        // The channel only closes once every sender has been dropped, which is
        // how the shutdown path in main.rs signals "no more avatars are coming."
        // Without this, anything still sitting in the buffer here (below the
        // flush threshold/interval) would be silently discarded on exit, even
        // though the local cache already marked those IDs as sent.
        if !self.buffer.is_empty()
            && let Err(err) = self.flush_buffer().await
        {
            error!("[{LOG_NAME}]: Failed to flush buffer on shutdown: {err}");
        }

        Ok(())
    }

    #[must_use]
    pub fn new(settings: &'s Settings) -> (Self, Sender<String>) {
        Self::new_with_base_url_and_flush_interval(
            settings,
            FLUSH_THRESHOLD,
            INGEST_BASE_URL.to_string(),
            FLUSH_INTERVAL,
        )
    }

    #[must_use]
    pub fn new_with_base_url_and_flush_interval(
        settings: &'s Settings,
        capacity: usize,
        base_url: String,
        flush_interval: Duration,
    ) -> (Self, Sender<String>) {
        let (tx, rx) = flume::bounded(capacity);

        (
            Self {
                settings,
                client: Client::default(),
                buffer: Vec::new(),
                base_url,
                channel: rx,
                flush_interval,
                last_flush: Instant::now(),
            },
            tx,
        )
    }

    /// # Errors
    /// Will return `Err` if anything errors
    pub async fn flush_buffer(&mut self) -> anyhow::Result<()> {
        let json = json!({
            "avatar_ids":  self.buffer,
            "attribution": self.settings.attribution.get_user_id().await,
        });

        let mut current_try = 0;
        let mut success = false;
        let mut ticket = None;
        while current_try < RETRY_LIMIT && !success {
            if current_try != 0 {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            debug!("[{LOG_NAME}] (try {current_try}) Sending {json:#?}");
            current_try += 1; // incrementing here to be able to cont later
            let response: reqwest::Response = self
                .client
                .post(Url::parse(&self.base_url)?.join("ingest")?)
                .header("User-Agent", USER_AGENT)
                .json(&json)
                .timeout(Duration::from_secs(5))
                .send()
                .await?;

            let status = response.status();
            let text = response.text().await?;
            success = match status {
                StatusCode::OK => true, // the API de-dupes already-known IDs
                StatusCode::TOO_MANY_REQUESTS => {
                    warn!("[{LOG_NAME}] 429 Rate Limit, trying again in 10 seconds");
                    false
                }
                _ => {
                    error!("[{LOG_NAME}] Unknown Error: {status} | {text}");
                    false
                }
            };
            debug!("[{LOG_NAME}] {status} | {text}");
            if !success {
                continue;
            }
            let data = serde_json::from_str::<IngestResponse>(&text)?;
            ticket = Some(data.ticket);
        }

        if current_try >= RETRY_LIMIT {
            bail!("[{LOG_NAME}] Failed after {current_try} retries to flush buffer, aborting");
        }

        if let Some(ticket) = ticket {
            debug!("[{LOG_NAME}] Ingest ticket: {ticket}");
        } else {
            let website = Link::new("KitsuneDB", WEBSITE_URL)
                .to_string()
                .color(Color::Red);
            error!(
                "[{LOG_NAME}] No ticket set - this is likely an error, please report at {website}"
            );
        }

        self.buffer.clear();
        self.last_flush = Instant::now();

        Ok(())
    }
}

impl KitsuneDB {
    #[must_use]
    pub const fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IngestResponse {
    avatars_enqueued: u64,
    invalid_ids:      u64,
    ticket:           String,
}

#[async_trait::async_trait]
impl Provider for KitsuneDB {
    fn kind(&self) -> ProviderKind {
        ProviderKind::KITSUNEDB
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        self.sender.send_async(avatar_id.to_string()).await?;
        Ok(true)
    }
}
