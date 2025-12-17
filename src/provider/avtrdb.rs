use std::{
    sync::{Arc, atomic::AtomicU64},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, bail};
use async_trait::async_trait;
use colored::{Color, Colorize};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use terminal_link::Link;

use crate::{
    USER_AGENT,
    provider::{Provider, ProviderKind},
    settings::Settings,
};

const INGEST_BASE_URL: &str = "https://api.avtrdb.com/v3/";

const DISCORD_URL: &str = "https://avtrdb.com/discord";

const FLUSH_INTERVAL: Duration = Duration::from_secs(2 * 60);
const FLUSH_THRESHOLD: usize = 100;

const RETRY_LIMIT: usize = 5;

#[derive(Clone)]
pub struct AvtrDB {
    settings: Arc<Settings>,
    client: Client,
    buffer: Arc<tokio::sync::RwLock<Vec<String>>>,
    base_url: String,
    last_flush: Arc<AtomicU64>,
}

impl AvtrDB {
    #[must_use]
    pub fn new(settings: Arc<Settings>) -> Self {
        Self::new_with_base_url_and_flush_interval(
            settings,
            INGEST_BASE_URL.to_string(),
            FLUSH_INTERVAL,
        )
    }

    #[must_use]
    pub fn new_with_base_url_and_flush_interval(
        settings: Arc<Settings>,
        base_url: String,
        flush_interval: Duration,
    ) -> Self {
        let instance = Self {
            settings,
            client: Client::default(),
            buffer: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            base_url,
            last_flush: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time to not run backwards")
                    .as_secs(),
            )),
        };

        let task_instance = instance.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(flush_interval).await;
                if let Err(err) = task_instance.flush_buffer().await {
                    error!("[AVTRDB] Periodic flush failed: {:?}", err);
                }
            }
        });

        instance
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IngestResponse {
    avatars_enqueued: u64,
    invalid_ids: u64,
    ticket: String,
}

#[async_trait]
impl Provider for AvtrDB {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AVTRDB
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        buffer.push(avatar_id.to_string());
        let buf_len = buffer.len();
        drop(buffer);

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let last = self.last_flush.load(std::sync::atomic::Ordering::Relaxed);

        // let should_flush =
        //     buf_len >= FLUSH_THRESHOLD || now.saturating_sub(last) >= FLUSH_INTERVAL.as_secs();

        if buf_len < FLUSH_THRESHOLD {
            return Ok(true);
        }

        if self
            .last_flush
            .compare_exchange(
                last,
                now,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            // Another task beat us to the punch
            return Ok(true);
        }

        self.flush_buffer().await?;

        Ok(true)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl AvtrDB {
    /// # Errors
    /// Will return `Err` if anything errors
    pub async fn flush_buffer(&self) -> anyhow::Result<()> {
        let kind = self.kind();
        let mut buffer = self.buffer.write().await;
        let json = json!({
            "avatar_ids":  *buffer,
            "attribution": self.settings.attribution.get_user_id(),
        });

        let mut current_try = 0;
        let mut success = false;
        let mut ticket = None;
        while current_try < RETRY_LIMIT && !success {
            if current_try != 0 {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            debug!("[{kind}] (try {current_try} Sending {json:#?}");
            current_try += 1; // incrementing here to be able to cont later
            println!("{}", self.base_url);
            let response = self
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
                    warn!("[{kind}] 429 Rate Limit, trying again in 10 seconds");
                    false
                }
                _ => {
                    error!("[{kind}] Unknown Error: {status} | {text}");
                    println!("[{kind}] Unknown Error: {status} | {text}");
                    false
                }
            };
            debug!("[{kind}] {status} | {text}");
            if !success {
                continue;
            }
            let data = serde_json::from_str::<IngestResponse>(&text)?;
            ticket = Some(data.ticket);
        }

        if current_try >= RETRY_LIMIT {
            bail!("[{kind}] Failed after {current_try} retires to flush buffer, aborting");
        }

        if let Some(ticket) = ticket {
            let check_status_url = format!("https://avtrdb.com/check_ticket_status/{ticket}",);
            let link = Link::new("here", &check_status_url)
                .to_string()
                .color(Color::Magenta); // the link is quite long, so i dont display it - can be changed
            info!("[{kind}] Check ingestion status {link}");
        } else {
            let discord = Link::new("discord", DISCORD_URL)
                .to_string()
                .color(Color::Red);
            error!("[{kind}] No ticket set - this is likely an error, please report in {discord}");
        }

        buffer.clear();

        // no idea why, as this will be dropped anyway on return. This makes clippy happy
        // only reasoning that maybe a lot of stuff will be droped and we will make sure
        // that is already been dealt with before the rest of the function call needs to
        // unwound
        drop(buffer);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httptest::{
        Expectation, Server, cycle,
        matchers::*,
        responders::{json_encoded, status_code},
    };
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_send_avatar_id_adds_to_buffer() {
        let settings = Arc::new(Settings::default());
        let provider = AvtrDB::new(settings);

        provider.send_avatar_id("avatar_1").await.unwrap();

        let buffer = provider.buffer.read().await;
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], "avatar_1");
        drop(buffer);
    }

    #[tokio::test]
    async fn test_flush_buffer_clears_buffer_on_success() {
        let server = Server::run();

        server.expect(
            Expectation::matching(request::method_path("POST", "/avatar/ingest")).respond_with(
                httptest::responders::json_encoded(serde_json::json!({
                    "avatars_enqueued": 1,
                    "invalid_ids": 0,
                    "ticket": "test_ticket"
                })),
            ),
        );

        let provider = AvtrDB::new_with_base_url_and_flush_interval(
            Arc::new(Settings::default()),
            server.url_str(""),
            Duration::from_secs(1),
        );

        // prefill buffer
        {
            let mut buf = provider.buffer.write().await;
            buf.push("avatar_1".to_string());
        }

        provider.flush_buffer().await.unwrap();

        let buffer = provider.buffer.read().await;
        assert!(buffer.is_empty());
        drop(buffer);
    }

    #[tokio::test]
    async fn test_send_avatar_id_triggers_flush_on_threshold() {
        let server = Server::run();

        server.expect(
            Expectation::matching(request::method_path("POST", "/avatar/ingest")).respond_with(
                httptest::responders::json_encoded(serde_json::json!({
                    "avatars_enqueued": 1,
                    "invalid_ids": 0,
                    "ticket": "ticket123"
                })),
            ),
        );

        let provider = AvtrDB::new_with_base_url_and_flush_interval(
            Arc::new(Settings::default()),
            server.url_str(""),
            Duration::from_secs(1),
        );

        {
            let mut buf = provider.buffer.write().await;
            buf.extend((0..FLUSH_THRESHOLD).map(|_| "avatar".to_string()));
        }

        let result = provider.send_avatar_id("avatar_new").await.unwrap();
        assert!(result);

        let buffer = provider.buffer.read().await;
        assert_eq!(buffer.len(), 0); // buffer should be cleared
        drop(buffer);
    }

    #[tokio::test]
    async fn test_periodic_flush_task_runs() {
        let server = Server::run();

        server.expect(
            Expectation::matching(request::method_path("POST", "/avatar/ingest")).respond_with(
                httptest::responders::json_encoded(serde_json::json!({
                    "avatars_enqueued": 1,
                    "invalid_ids": 0,
                    "ticket": "ticket_periodic"
                })),
            ),
        );

        let provider = AvtrDB::new_with_base_url_and_flush_interval(
            Arc::new(Settings::default()),
            server.url_str(""),
            Duration::from_secs(1),
        );

        // add one avatar to buffer
        provider.send_avatar_id("avatar_1").await.unwrap();

        // wait slightly longer than flush interval
        sleep(Duration::from_secs(2)).await;

        let buffer = provider.buffer.read().await;
        assert!(buffer.is_empty());
        drop(buffer);
    }

    #[tokio::test]
    async fn test_retry_on_failure() {
        let server = Server::run();
        let base_url = server.url_str("");

        server.expect(
            Expectation::matching(request::method_path("POST", "/avatar/ingest"))
                .times(1)
                .respond_with(status_code(500)),
        );

        server.expect(
            Expectation::matching(request::method_path("POST", "/avatar/ingest"))
                .times(1)
                .respond_with(json_encoded(serde_json::json!({
                    "avatars_enqueued": 1,
                    "invalid_ids": 0,
                    "ticket": "ticket_retry"
                }))),
        );
        let provider = AvtrDB::new_with_base_url_and_flush_interval(
            Arc::new(Settings::default()),
            base_url.to_string(), // Use the base_url
            Duration::from_secs(1),
        );

        {
            let mut buf = provider.buffer.write().await;
            buf.push("avatar_1".to_string());
        }

        provider.flush_buffer().await.unwrap();

        // Verify buffer cleared
        let buffer = provider.buffer.read().await;
        assert!(buffer.is_empty());
        drop(buffer);
    }
}
