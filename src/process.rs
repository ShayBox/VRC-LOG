use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "cache")]
use crate::cache;
use crate::{print_colorized, provider::Provider};

#[cfg(feature = "cache")]
pub async fn process_with_cache<I: IntoIterator<Item = String>>(
    providers: Vec<Arc<Box<dyn Provider>>>,
    cache: &cache::Cache,
    avatar_ids: I,
) -> anyhow::Result<()> {
    let checked_ids = cache
        .check_all_ids(avatar_ids)
        .await?
        .into_iter()
        .collect::<Vec<_>>();

    let (tx, rx) = flume::unbounded();
    let checked_ids = Arc::new(checked_ids);
    let mut base_bits = HashMap::new();

    for (id, provider_bits) in checked_ids.iter() {
        print_colorized(id);
        base_bits.insert(id.clone(), *provider_bits);
    }

    for provider in &providers {
        let provider = provider.clone();
        let tx_clone = tx.clone();
        let checked_ids = checked_ids.clone();
        tokio::spawn(async move {
            let kind = provider.kind();
            let kind_bit = kind as u32;
            for (id, provider_bits) in checked_ids.iter() {
                if provider_bits & kind_bit != 0 {
                    continue;
                }
                match provider.send_avatar_id(id).await {
                    Ok(success) => {
                        if success {
                            info!("^ Successfully Submitted to {kind}");
                            let _ = tx_clone.send_async((id.clone(), kind_bit)).await;
                        }
                    }
                    Err(err) => {
                        error!("^ Failed to submit to {kind}: {err}");
                    }
                }
            }
        });
    }

    drop(tx);

    let mut updated_bits = HashMap::new();

    while let Ok((id, kind_bit)) = rx.recv_async().await {
        let entry = updated_bits.entry(id).or_insert(0);
        *entry |= kind_bit;
    }

    let mut buffer = Vec::new();
    for (id, bits) in base_bits {
        if let Some(add_bits) = updated_bits.get(&id) {
            buffer.push((id, bits | add_bits));
        }
    }

    cache.store_avatar_ids_with_providers(buffer).await
}

#[cfg(not(feature = "cache"))]
pub async fn process_without_cache<I: IntoIterator<Item = String>>(
    providers: Vec<Arc<Box<dyn Provider>>>,
    avatar_ids: I,
) -> anyhow::Result<()> {
    for avatar_id in avatar_ids {
        print_colorized(&avatar_id);

        // Collect all provider futures for this avatar_id
        let futures = providers
            .iter()
            .map(|provider| provider.send_avatar_id(&avatar_id));
        let results = futures::future::join_all(futures).await;

        for (provider, result) in providers.iter().zip(results) {
            let kind = provider.kind();
            match result {
                Ok(unique) => {
                    if unique {
                        info!("^ Successfully Submitted to {kind}");
                    }
                }
                Err(error) => {
                    error!("^ Failed to submit to {kind}: {error}");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use async_trait::async_trait;
    use strum::IntoEnumIterator;
    use tokio::sync::Mutex;

    use super::*;
    use crate::provider::ProviderKind;

    #[derive(Clone)]
    struct MockProvider {
        kind:    ProviderKind,
        sent:    Arc<Mutex<Vec<String>>>,
        succeed: bool,
    }

    impl MockProvider {
        #[allow(clippy::new_ret_no_self, clippy::type_complexity)]
        fn new(
            kind: ProviderKind,
            succeed: bool,
        ) -> (Arc<Box<dyn Provider>>, Arc<Mutex<Vec<String>>>) {
            let sent = Arc::new(Mutex::new(Vec::new()));

            let provider = Self {
                kind,
                sent: sent.clone(),
                succeed,
            };

            (Arc::new(Box::new(provider)), sent)
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn kind(&self) -> ProviderKind {
            self.kind
        }

        async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
            self.sent.lock().await.push(avatar_id.to_string());
            Ok(self.succeed)
        }
    }

    #[cfg(not(feature = "cache"))]
    #[tokio::test]
    async fn process_without_cache_sends_to_all_providers() -> Result<()> {
        let (provider_a, sent_a) = MockProvider::new(ProviderKind::AVTRDB, true);
        let (provider_b, sent_b) = MockProvider::new(ProviderKind::NSVR, true);

        let providers = vec![provider_a, provider_b];

        let avatar_ids = vec!["avtr_1".to_string(), "avtr_2".to_string()];

        process_without_cache(providers, avatar_ids).await?;

        let sent_a = sent_a.lock().await;
        let sent_b = sent_b.lock().await;

        assert_eq!(sent_a.len(), 2);
        assert_eq!(sent_b.len(), 2);

        Ok(())
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn process_with_cache_skips_existing_provider_bits() -> Result<()> {
        let (provider_a, sent_a) = MockProvider::new(ProviderKind::AVTRDB, true);
        let (provider_b, sent_b) = MockProvider::new(ProviderKind::NSVR, true);

        let providers = vec![provider_a, provider_b];

        let cache = cache::Cache::new_in_memory().await?;

        // Pre-seed cache: AVTRDB already handled this avatar
        cache
            .store_avatar_ids_with_providers(
                vec![("avtr_1", ProviderKind::AVTRDB as u32)].into_iter(),
            )
            .await?;

        let avatar_ids = vec!["avtr_1".to_string()];
        process_with_cache(providers, &cache, avatar_ids).await?;

        let sent_a = sent_a.lock().await;
        let sent_b = sent_b.lock().await;

        assert_eq!(sent_a.len(), 0, "AVTRDB must be skipped");
        assert_eq!(sent_b.len(), 1, "NSVR must be called");
        drop(sent_a);
        drop(sent_b);

        Ok(())
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn process_with_cache_updates_provider_bits_on_success() -> Result<()> {
        let (provider, _sent) = MockProvider::new(ProviderKind::PAW, true);
        let providers = vec![provider];

        let cache = cache::Cache::new_in_memory().await?;

        let avatar_ids = vec!["avtr_42".to_string()];

        process_with_cache(providers, &cache, avatar_ids).await?;

        let result = cache
            .check_all_ids(vec!["avtr_42".to_string()].into_iter())
            .await?;

        let bits = result["avtr_42"];
        assert!(bits & ProviderKind::PAW as u32 != 0);

        Ok(())
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn failed_provider_does_not_update_cache() -> Result<()> {
        let (provider, sent) = MockProvider::new(ProviderKind::VRCDB, false);
        let providers = vec![provider];

        let cache = cache::Cache::new_in_memory().await?;

        let avatar_ids = vec!["avtr_fail".to_string()];

        process_with_cache(providers, &cache, avatar_ids).await?;

        let sent = sent.lock().await;
        assert_eq!(sent.len(), 1, "Provider should still be called");
        drop(sent);

        let result = cache
            .check_all_ids(vec!["avtr_fail".to_string()].into_iter())
            .await?;

        assert!(result["avtr_fail"] == 0, "Cache must not update on failure");

        Ok(())
    }

    #[test]
    fn provider_kind_bitmask_is_unique() {
        let kinds = ProviderKind::iter().collect::<Vec<_>>();

        for (i, a) in kinds.iter().enumerate() {
            for b in kinds.iter().skip(i + 1) {
                assert_eq!(
                    (*a as u32) & (*b as u32),
                    0,
                    "ProviderKind bits must not overlap"
                );
            }
        }
    }

    #[test]
    fn provider_bit_check_logic() {
        let bits = ProviderKind::AVTRDB as u32 | ProviderKind::NSVR as u32;

        assert!(bits & ProviderKind::AVTRDB as u32 != 0);
        assert!(bits & ProviderKind::NSVR as u32 != 0);
        assert!(bits & ProviderKind::PAW as u32 == 0);
    }
}
