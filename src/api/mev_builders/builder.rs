use std::str::FromStr;

use alloy::signers::Signer;
use alloy::{
    network::EthereumWallet,
    signers::local::{LocalSigner, PrivateKeySigner},
};
use lazy_static::lazy_static;
use revm::primitives::keccak256;
use serde_json::json;

use crate::api::mev_builders::types::{BundleResponse, BundleStatsResponse};

use super::types::{EndOfBlockBundleParams, SendBundleParams};

lazy_static! {
    // ***REMOVED*** Leaked Key (***REMOVED***)
    // pub static ref WHITELISTED_SIGNER: PrivateKeySigner =
    //     "***REMOVED***"
    //         .parse()
    //         .unwrap();
    pub static ref WHITELISTED_SIGNER: PrivateKeySigner =
    "f626163a97da276675ecee20948940f78c7ca602cf0a68c3b1acc04c8babfccd"
        .parse()
        .unwrap();
}

pub struct Builder {
    pub name: String,
    pub has_statistics_api: bool,
    pub custom_default_send_bundle_method: Option<String>,
    rpc_url: String,
    end_of_block_method: Option<String>,
}

impl Builder {
    pub fn new(rpc_url: &str, has_statistics_api: bool) -> Builder {
        let name = Builder::extract_name(rpc_url);
        return Builder {
            name,
            rpc_url: rpc_url.to_string(),
            end_of_block_method: None,
            has_statistics_api: has_statistics_api,
            custom_default_send_bundle_method: None,
        };
    }

    pub fn new_with_custom_method(
        rpc_url: &str,
        has_statistics_api: bool,
        custom_method: String,
    ) -> Builder {
        let name = Builder::extract_name(rpc_url);
        return Builder {
            name,
            rpc_url: rpc_url.to_string(),
            end_of_block_method: None,
            has_statistics_api: has_statistics_api,
            custom_default_send_bundle_method: Some(custom_method),
        };
    }

    pub fn new_with_eob(rpc_url: &str, eob_method: &str, has_statistics_api: bool) -> Builder {
        let name = Builder::extract_name(rpc_url);
        return Builder {
            name,
            rpc_url: rpc_url.to_string(),
            end_of_block_method: Some(eob_method.to_string()),
            has_statistics_api: has_statistics_api,
            custom_default_send_bundle_method: None,
        };
    }

    fn extract_name(rpc_url: &str) -> String {
        let binding = rpc_url.replace("https://", "").replace("http://", "");

        let url = binding.split('.').collect::<Vec<&str>>();

        let base = if url.len() > 2 { url[1] } else { url[0] };

        base.to_string()
    }

    pub async fn send_bundle(
        &self,
        mut params: SendBundleParams,
    ) -> Result<BundleResponse, Box<dyn std::error::Error>> {
        params.txs = sanitize_txs(params.txs);
        params.block_number = sanitize_block_number(params.block_number);

        let method = {
            if let Some(ref method) = &self.custom_default_send_bundle_method {
                method
            } else {
                &"eth_sendBundle".to_string()
            }
        };

        let payload = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "method": method,
          "params": [params]
        });

        let payload_str = serde_json::to_string(&payload)?;

        let sig_hex = sign_message(&payload_str, &WHITELISTED_SIGNER)
            .await
            .unwrap();

        let client = reqwest::Client::new();

        let response = client
            .post(&self.rpc_url)
            .header(
                "X-Flashbots-Signature",
                format!("{}:0x{}", WHITELISTED_SIGNER.address(), sig_hex),
            )
            .header("Content-Type", "application/json")
            .body(payload_str)
            .send()
            .await?;

        let text = response.text().await.unwrap();

        let response_json: BundleResponse = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(err) => {
                if !self.has_statistics_api {
                    BundleResponse {
                        id: 1,
                        jsonrpc: "-1".to_string(),
                        error: None,
                        result: None,
                    }
                } else {
                    println!("{:?} | {}", text, self.name);
                    return Err(err.into());
                }
            }
        };

        Ok(response_json)
    }

    pub async fn send_end_of_block_bundle(
        &self,
        mut params: EndOfBlockBundleParams,
    ) -> Result<BundleResponse, Box<dyn std::error::Error>> {
        params.txs = sanitize_txs(params.txs);
        params.block_number = sanitize_block_number(params.block_number);

        let eob_method = match &self.end_of_block_method {
            Some(method) => method,
            None => {
                return Err("Tried to call send_end_of_block_bundle for {}, however the builder->end_of_block_method is empty.".into());
            }
        };

        let payload = json!({
          "jsonrpc": "2.0",
          "id": 1,
          "method": eob_method,
          "params": [params]
        });

        let payload_str = serde_json::to_string(&payload)?;

        let sig_hex = sign_message(&payload_str, &WHITELISTED_SIGNER)
            .await
            .unwrap();

        let client = reqwest::Client::new();

        let response = client
            .post(&self.rpc_url)
            .header(
                "X-Flashbots-Signature",
                format!("{}:0x{}", WHITELISTED_SIGNER.address(), sig_hex),
            )
            .header("Content-Type", "application/json")
            .body(payload_str.clone())
            .send()
            .await?;

        let response_text = response.text().await.unwrap();
        let response_json: BundleResponse = match serde_json::from_str(&response_text) {
            Ok(r) => r,
            Err(err) => {
                println!(
                    "Error decoding as BundleResponse: {:?} | {} | {}",
                    &response_text, self.name, err
                );
                return Err(err.into());
            }
        };

        Ok(response_json)
    }

    pub async fn get_bundle_receipt(
        &self,
        bundle_hash: &str,
        block_number: u64,
    ) -> Result<BundleStatsResponse, Box<dyn std::error::Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "flashbots_getBundleStats",
            "params": [json!({
                "bundleHash": bundle_hash,
                "blockNumber": format!("0x{:x}", block_number)
            })]
        });

        let payload_str = serde_json::to_string(&payload)?;

        let sig_hex = sign_message(&payload_str, &WHITELISTED_SIGNER)
            .await
            .unwrap();

        let client = reqwest::Client::new();

        let response = client
            .post(&self.rpc_url)
            .header(
                "X-Flashbots-Signature",
                format!("{}:0x{}", WHITELISTED_SIGNER.address(), sig_hex),
            )
            .header("Content-Type", "application/json")
            .body(payload_str)
            .send()
            .await?;

        // println!("{}", response.text().await.unwrap());
        // return Err("asdopdaospoasdp".into());

        let response_json: BundleStatsResponse = response.json().await?;
        println!("{:?}", response_json);

        Ok(response_json)
    }

    pub fn supports_eob(&self) -> bool {
        return self.end_of_block_method.is_some();
    }
}

async fn sign_message(
    message: &str,
    signer: &PrivateKeySigner,
) -> Result<String, Box<dyn std::error::Error>> {
    let sig = signer
        .sign_message(
            format!(
                "0x{:x}",
                alloy::primitives::B256::from(keccak256(message.as_bytes()))
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    Ok(hex::encode(sig.as_bytes()))
}

fn sanitize_txs(txs: Vec<String>) -> Vec<String> {
    txs.into_iter()
        .map(|tx| {
            if tx.starts_with("0x") || tx.starts_with("0X") {
                tx
            } else {
                let mut sanitized_tx = String::with_capacity(tx.len() + 2);
                sanitized_tx.push_str("0x");
                sanitized_tx.push_str(&tx);
                sanitized_tx
            }
        })
        .collect()
}

fn sanitize_block_number(maybe_block_number: Option<String>) -> Option<String> {
    if let Some(ref block_number) = maybe_block_number {
        if !block_number.starts_with("0x") {
            return Some(format!("0x{}", block_number));
        }
    }
    return maybe_block_number;
}
