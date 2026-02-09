use std::time::Duration;

use anyhow::{bail, Result};
use colored::{Color, Colorize};
use flume::{Receiver, Sender};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use terminal_link::Link;
use tokio::time::Instant;

use crate::{
    provider::{Provider, ProviderKind},
    settings::Settings,
    USER_AGENT,
};

const INGEST_BASE_URL: &str = "https://api.avtrdb.com/v3/";

const DISCORD_URL: &str = "https://avtrdb.com/discord";

const FLUSH_INTERVAL: Duration = Duration::from_secs(2 * 60);
const FLUSH_THRESHOLD: usize = 100;

const RETRY_LIMIT: usize = 5;

const LOG_NAME: &str = "avtrDB";

pub struct AvtrDB {
    sender: Sender<String>,
}

pub struct AvtrDBActor<'s> {
    settings:       &'s Settings,
    client:         Client,
    buffer:         Vec<String>,
    base_url:       String,
    channel:        Receiver<String>,
    flush_interval: Duration,
    last_flush:     Instant,
}

impl<'s> AvtrDBActor<'s> {
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
        let (rx, tx) = flume::bounded(capacity);

        (
            Self {
                settings,
                client: Client::default(),
                buffer: Vec::new(),
                base_url,
                channel: tx,
                flush_interval,
                last_flush: Instant::now(),
            },
            rx,
        )
    }

    /// # Errors
    /// Will return `Err` if anything errors
    pub async fn flush_buffer(&mut self) -> anyhow::Result<()> {
        let json = json!({
            "avatar_ids":  self.buffer,
            "attribution": self.settings.attribution.get_user_id(),
        });

        let mut current_try = 0;
        let mut success = false;
        let mut ticket = None;
        while current_try < RETRY_LIMIT && !success {
            if current_try != 0 {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            debug!("[{LOG_NAME}] (try {current_try} Sending {json:#?}");
            current_try += 1; // incrementing here to be able to cont later
            let response: reqwest::Response = self
                .client
                .post(Url::parse(&self.base_url)?.join("avatar/ingest")?)
                .header("User-Agent", USER_AGENT)
                .json(&json)
                .timeout(Duration::from_secs(5))
                .send()
                .await?;

            let status = response.status();
            let text = response.text().await?;
            success = match status {
                StatusCode::OK => true, // the API checks for things already enqueued
                StatusCode::TOO_MANY_REQUESTS => {
                    warn!("[{LOG_NAME}] 429 Rate Limit, trying again in 10 seconds");
                    false
                }
                _ => {
                    error!("[{LOG_NAME}] Unknown Error: {status} | {text}");
                    println!("[{LOG_NAME}] Unknown Error: {status} | {text}");
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
            bail!("[{LOG_NAME}] Failed after {current_try} retires to flush buffer, aborting");
        }

        if let Some(ticket) = ticket {
            let check_status_url = format!("https://avtrdb.com/check_ticket_status/{ticket}",);
            let link = Link::new("here", &check_status_url)
                .to_string()
                .color(Color::Magenta); // the link is quite long, so i dont display it - can be changed
            info!("[{LOG_NAME}] Check ingestion status {link}");
        } else {
            let discord = Link::new("discord", DISCORD_URL)
                .to_string()
                .color(Color::Red);
            error!(
                "[{LOG_NAME}] No ticket set - this is likely an error, please report in {discord}"
            );
        }

        self.buffer.clear();
        self.last_flush = Instant::now();

        Ok(())
    }
}

impl AvtrDB {
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
impl Provider for AvtrDB {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AVTRDB
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        self.sender.send_async(avatar_id.to_string()).await?;
        Ok(true)
    }
}
