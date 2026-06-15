//! CMC endpoint paths in one place so the REST client stays declarative.

pub const BASE_URL: &str = "https://pro-api.coinmarketcap.com";

pub const LATEST_QUOTES: &str = "/v2/cryptocurrency/quotes/latest";
pub const OHLCV: &str = "/v2/cryptocurrency/ohlcv/historical";
pub const FEAR_GREED: &str = "/v3/fear-and-greed/latest";
pub const TRENDING: &str = "/v1/cryptocurrency/trending/latest";
pub const DEX_LIQUIDITY: &str = "/v4/dex/pairs/quotes/latest";
pub const GLOBAL_METRICS: &str = "/v1/global-metrics/quotes/latest";
