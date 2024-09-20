use revm::primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendBundleParams {
    pub txs: Vec<String>, // Array[String], A list of signed transactions to execute in an atomic bundle, list can be empty for bundle cancellations
    #[serde(skip_serializing_if = "Option::is_none", rename = "blockNumber")]
    pub block_number: Option<String>, // (Optional) String, a hex-encoded block number for which this bundle is valid. Default, current block number
    #[serde(skip_serializing_if = "Option::is_none", rename = "revertingTxHashes")]
    pub reverting_tx_hashes: Option<Vec<revm::primitives::FixedBytes<32>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "droppingTxHashes")]
    pub dropping_tx_hashes: Option<Vec<revm::primitives::FixedBytes<32>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "replacementUuid")]
    pub replacement_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "refundPercent")]
    pub refund_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "refundIndex")]
    pub refund_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "refundRecipient")]
    pub refund_recipient: Option<Address>, // (Optional) Address, the address that will receive the ETH refund. Default, sender of the first transaction in the bundle
}

impl Default for SendBundleParams {
    fn default() -> SendBundleParams {
        SendBundleParams {
            txs: vec![],
            block_number: None,
            reverting_tx_hashes: None,
            dropping_tx_hashes: None,
            replacement_uuid: None,
            refund_percent: None,
            refund_index: None,
            refund_recipient: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndOfBlockBundleParams {
    pub txs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "blockNumber")]
    pub block_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "revertingTxHashes")]
    pub reverting_tx_hashes: Option<Vec<revm::primitives::FixedBytes<32>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "targetPools")]
    pub target_pools: Option<Vec<Address>>,
}

impl Default for EndOfBlockBundleParams {
    fn default() -> EndOfBlockBundleParams {
        EndOfBlockBundleParams {
            txs: vec![],
            block_number: None,
            reverting_tx_hashes: None,
            target_pools: None
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BundleResponse {
    pub jsonrpc: String,
    pub id: usize,
    pub result: Option<BundleResult>,
    pub error: Option<BundleError>,
}

#[derive(Debug, Deserialize)]
pub struct BundleResult {
    #[serde(rename = "bundleHash")]
    pub bundle_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct BundleError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct BundleStatsResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<BundleStatsResult>,
    pub error: Option<BundleError>,
}

#[derive(Debug, Deserialize)]
pub struct BundleStatsResult {
    #[serde(rename = "isSimulated")]
    pub is_simulated: bool,
    #[serde(rename = "isHighPriority")]
    pub is_high_priority: bool,
    #[serde(rename = "simulatedAt")]
    pub simulated_at: String,
    #[serde(rename = "submittedAt")]
    pub submitted_at: String,
    #[serde(rename = "consideredByBuildersAt")]
    pub considered_by_builders_at: Option<String>,
}