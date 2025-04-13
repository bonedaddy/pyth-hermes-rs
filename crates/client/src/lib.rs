//! Rust library for querying deployments of the Pyth Hermes API

pub mod types;

use {
    futures_util::StreamExt,
    reqwest::{Client, Error},
    reqwest_eventsource::{Error as EventSourceError, Event, EventSource},
    std::sync::Arc,
    tokio::task::JoinHandle,
    types::*,
};

pub struct HermesClient {
    http: reqwest::Client,
    base_url: Arc<str>,
}

impl HermesClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: Arc::from(base_url.into()),
        }
    }

    /// Get the latest price updates by price feed id.
    pub async fn get_latest_price_feeds(&self, ids: &[&str]) -> Result<Vec<RpcPriceFeed>, Error> {
        let url = format!("{}/v2/updates/price/latest", self.base_url);
        let mut req = self.http.get(&url);
        for id in ids {
            req = req.query(&[("ids[]", *id)]);
        }
        let resp = req.send().await?.error_for_status()?;
        let feeds = resp.json::<PriceUpdate>().await?;
        Ok(feeds.parsed.unwrap_or_default())
    }

    /// This endpoint fetches all price feeds from the Pyth network. It can be filtered by asset type and query string.
    ///
    /// # Arguments
    ///
    /// * `query` - If provided results will be filtered for price feeds whose symbol contains the query string
    /// * `asset_type` - If provides filter by asset type. Values are crypto, equity, fx, metal, rates
    pub async fn get_price_feeds_metadata(
        &self,
        query: Option<&str>,
        asset_type: Option<&str>,
    ) -> Result<Vec<PriceFeedMetadata>, Error> {
        let url = format!("{}/v2/price_feeds", self.base_url);
        let req = self
            .http
            .get(&url)
            .query(&[("query", query), ("asset_type", asset_type)]);
        let resp = req.send().await?.error_for_status()?;
        resp.json::<Vec<PriceFeedMetadata>>().await
    }

    /// Get the latest price updates by price feed id, with a publish time greater than `publish_time`
    ///
    /// # Arguments
    ///
    /// * `publish_time` - Only return price feed updates that are greater than or equal to this timestamp
    pub async fn get_price_updates_by_time(
        &self,
        publish_time: i64,
        ids: &[&str],
    ) -> Result<PriceUpdate, Error> {
        let url = format!("{}/v2/updates/price/{}", self.base_url, publish_time);
        let mut req = self.http.get(&url);
        for id in ids {
            req = req.query(&[("ids[]", *id)]);
        }
        let resp = req.send().await?.error_for_status()?;
        resp.json::<PriceUpdate>().await
    }

    /// Get the latest TWAP by price feed id with a custom time window.
    ///
    /// # Arguments
    /// * `window_seconds` - Time period in seconds used to calculate the TWAP, ending at current time
    pub async fn get_latest_twaps(
        &self,
        window_seconds: u64,
        ids: &[&str],
    ) -> Result<TwapsResponse, Error> {
        let url = format!(
            "{}/v2/updates/twap/{}/latest",
            self.base_url, window_seconds
        );
        let mut req = self.http.get(&url);
        for id in ids {
            req = req.query(&[("ids[]", *id)]);
        }
        let resp = req.send().await?.error_for_status()?;
        resp.json::<TwapsResponse>().await
    }

    /// Gets the most recent publisher stake caps update data
    pub async fn get_latest_publisher_stake_caps(
        &self,
    ) -> Result<LatestPublisherStakeCapsUpdateDataResponse, Error> {
        let url = format!("{}/v2/updates/publisher_stake_caps/latest", self.base_url);
        let resp = self.http.get(&url).send().await?.error_for_status()?;
        resp.json::<LatestPublisherStakeCapsUpdateDataResponse>()
            .await
    }
    /// Spawns a task which streams price updates from the hermes api
    ///
    /// # Returns
    ///
    /// [`JoinHandle`] which can be used to abort the spawned task
    pub async fn stream_price_updates<F>(
        &self,
        ids: Vec<String>,
        mut on_event: F,
    ) -> Result<JoinHandle<()>, Error>
    where
        F: FnMut(ParsedPriceUpdate) + Send + 'static,
    {
        let base_url = self.base_url.clone();
        let client = self.http.clone();

        let handler = tokio::spawn(async move {
            loop {
                let url = format!("{}/v2/updates/price/stream", base_url);
                let mut req = client.get(&url);
                for id in &ids {
                    req = req.query(&[("ids[]", id)]);
                }

                let mut es = match EventSource::new(req) {
                    Ok(stream) => stream,
                    Err(err) => {
                        log::error!("failed to connect SSE {err:#?}");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };

                while let Some(event) = es.next().await {
                    match event {
                        Ok(Event::Message(msg)) => {
                            if let Ok(update) = serde_json::from_str::<PriceUpdate>(&msg.data) {
                                if let Some(parsed) = update.parsed {
                                    for item in parsed {
                                        if let Some(metadata) = item.metadata.clone() {
                                            let parsed_update = ParsedPriceUpdate {
                                                id: item.id,
                                                price: item.price,
                                                ema_price: item.ema_price,
                                                metadata,
                                            };
                                            on_event(parsed_update);
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Event::Open) => {
                            // Connection established
                        }
                        Err(EventSourceError::StreamEnded) => {
                            log::error!("stream ended, reconnecting");
                            break;
                        }
                        Err(err) => {
                            log::error!("sse error {err:#?}");
                            break;
                        }
                    }
                }
            }
        });

        Ok(handler)
    }
}

#[cfg(test)]
mod test {
    use super::{types::PUBLIC_BASE_URL, *};

    const ETH_USD_FEED_ID: &str =
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
    const SOL_USD_FEED_ID: &str =
        "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stream_price_updates_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let handler = client
            .stream_price_updates(
                vec![ETH_USD_FEED_ID.to_string(), SOL_USD_FEED_ID.to_string()],
                move |update| {
                    let _ = tx.send(update);
                },
            )
            .await
            .expect("Failed to start SSE stream");
        let mut found_eth_feed = false;
        let mut found_sol_feed = false;
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(20));
        timer.tick().await;
        loop {
            tokio::select! {
                result = rx.recv() => {
                    if let Some(update) = result {
                        println!("update {update:#?}");
                        if update.id.contains(ETH_USD_FEED_ID) {
                            found_eth_feed = true;
                        }
                        if update.id.contains(SOL_USD_FEED_ID) {
                            found_sol_feed = true;
                        }
                        if found_eth_feed && found_sol_feed {
                            break;
                        }
                    } else {
                        panic!("channel closed");
                    }
                }
                _ = timer.tick() => {
                    break;
                }
            }
        }
        handler.abort();
        if !found_eth_feed || !found_sol_feed {
            panic!("failed to find feeds");
        }
    }

    #[tokio::test]
    async fn test_latest_price_feeds() {
        let hc = HermesClient::new(PUBLIC_BASE_URL);

        let _ = hc.get_latest_price_feeds(&[ETH_USD_FEED_ID]).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest_price_feeds_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let result = client
            .get_latest_price_feeds(&[ETH_USD_FEED_ID])
            .await
            .unwrap();
        assert!(!result.is_empty());
        assert_eq!(
            result[0].id.to_lowercase(),
            ETH_USD_FEED_ID.trim_start_matches("0x").to_lowercase()
        );
    }

    #[tokio::test]
    async fn test_get_price_feeds_metadata_live_empty() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let metadata = client.get_price_feeds_metadata(None, None).await.unwrap();
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_price_feeds_metadata_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let metadata = client
            .get_price_feeds_metadata(Some("bitcoin"), None)
            .await
            .unwrap();
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_publisher_stake_caps_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let response = client.get_latest_publisher_stake_caps().await.unwrap();
        assert!(!response.binary.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_price_updates_by_time_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let result = client
            .get_price_updates_by_time(1717632000, &[ETH_USD_FEED_ID])
            .await;
        assert!(result.is_ok() || matches!(result, Err(reqwest::Error { .. })));
    }

    #[tokio::test]
    async fn test_get_latest_twaps_live() {
        let client = HermesClient::new(PUBLIC_BASE_URL);
        let result = client.get_latest_twaps(300, &[ETH_USD_FEED_ID]).await;
        assert!(result.is_ok() || matches!(result, Err(reqwest::Error { .. })));
    }
}
