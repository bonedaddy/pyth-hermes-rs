use serde::Deserialize;
use std::collections::HashMap;

/// URL of the public hermes api
pub const PUBLIC_BASE_URL: &str = "https://hermes.pyth.network";

#[derive(Debug, Deserialize)]
pub struct RpcPriceFeed {
    pub id: String,
    pub price: RpcPrice,
    pub ema_price: RpcPrice,
    pub metadata: Option<RpcPriceFeedMetadata>,
    pub vaa: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcPrice {
    pub price: String,
    pub conf: String,
    pub expo: i32,
    pub publish_time: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcPriceFeedMetadata {
    pub emitter_chain: Option<i32>,
    pub prev_publish_time: Option<i64>,
    pub price_service_receive_time: Option<i64>,
    pub slot: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PriceFeedMetadata {
    pub id: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct PriceUpdate {
    pub binary: BinaryUpdate,
    pub parsed: Option<Vec<RpcPriceFeed>>,
}

#[derive(Debug, Deserialize)]
pub struct BinaryUpdate {
    pub encoding: String,
    pub data: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwapsResponse {
    pub binary: BinaryUpdate,
    pub parsed: Option<Vec<ParsedPriceFeedTwap>>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedPriceFeedTwap {
    pub id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub twap: RpcPrice,
    pub down_slots_ratio: String,
}

#[derive(Debug, Deserialize)]
pub struct LatestPublisherStakeCapsUpdateDataResponse {
    pub binary: BinaryUpdate,
    pub parsed: Option<Vec<ParsedPublisherStakeCapsUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedPublisherStakeCapsUpdate {
    pub publisher_stake_caps: Vec<ParsedPublisherStakeCap>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedPublisherStakeCap {
    pub publisher: String,
    pub cap: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParsedPriceUpdate {
    pub id: String,
    pub price: RpcPrice,
    pub ema_price: RpcPrice,
    pub metadata: RpcPriceFeedMetadata,
}

impl RpcPrice {
    /// Converts the pyth reported price from an integer into a floating point
    pub fn to_f64(&self) -> Option<f64> {
        let price = self.price.parse::<u64>().ok()?;
        Some(price as f64 / ((10_u64.pow(self.expo.unsigned_abs())) as f64))
    }
}

#[cfg(test)]
mod test {
    use super::RpcPrice;

    #[test]
    fn test_rpc_price_to_f64() {
        let price = RpcPrice {
            price: "12971500000".to_string(),
            conf: "6486733".to_string(),
            expo: -8,
            publish_time: 1744523548,
        };
        assert_eq!(price.to_f64().unwrap(), 129.715);
        let price = RpcPrice {
            price: "160644665033".to_string(),
            conf: "73725033".to_string(),
            expo: -8,
            publish_time: 1744523627,
        };
        assert_eq!(price.to_f64().unwrap(), 1606.44665033)
    }
}
