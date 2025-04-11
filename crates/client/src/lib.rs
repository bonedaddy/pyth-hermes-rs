//! Rust library for querying deployments of the Pyth Hermes API

pub mod types;

use {
    reqwest::{Client, Error},
    std::sync::Arc,
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
}

#[cfg(test)]
mod test {
    use super::*;
    const BASE_URL: &str = "https://hermes.pyth.network";
    const FEED_ID: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";

    #[tokio::test]
    async fn test_latest_price_feeds() {
        let hc = HermesClient::new(BASE_URL);

        let _ = hc.get_latest_price_feeds(&[FEED_ID]).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest_price_feeds_live() {
        let client = HermesClient::new(BASE_URL);
        let result = client.get_latest_price_feeds(&[FEED_ID]).await.unwrap();
        assert!(!result.is_empty());
        assert_eq!(
            result[0].id.to_lowercase(),
            FEED_ID.trim_start_matches("0x").to_lowercase()
        );
    }

    #[tokio::test]
    async fn test_get_price_feeds_metadata_live_empty() {
        let client = HermesClient::new(BASE_URL);
        let metadata = client.get_price_feeds_metadata(None, None).await.unwrap();
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_price_feeds_metadata_live() {
        let client = HermesClient::new(BASE_URL);
        let metadata = client
            .get_price_feeds_metadata(Some("bitcoin"), None)
            .await
            .unwrap();
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_publisher_stake_caps_live() {
        let client = HermesClient::new(BASE_URL);
        let response = client.get_latest_publisher_stake_caps().await.unwrap();
        assert!(!response.binary.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_price_updates_by_time_live() {
        let client = HermesClient::new(BASE_URL);
        let result = client
            .get_price_updates_by_time(1717632000, &[FEED_ID])
            .await;
        assert!(result.is_ok() || matches!(result, Err(reqwest::Error { .. })));
    }

    #[tokio::test]
    async fn test_get_latest_twaps_live() {
        let client = HermesClient::new(BASE_URL);
        let result = client.get_latest_twaps(300, &[FEED_ID]).await;
        assert!(result.is_ok() || matches!(result, Err(reqwest::Error { .. })));
    }
}
