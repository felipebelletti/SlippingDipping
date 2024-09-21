use std::collections::HashMap;

use futures::{stream::FuturesUnordered, StreamExt};

use super::{
    builder::Builder,
    types::{BundleResult, EndOfBlockBundleParams, SendBundleParams},
};

pub async fn broadcast_bundle(
    params: SendBundleParams,
    builders: Vec<&'static Builder>,
) -> HashMap<String, Result<BundleResult, String>> {
    let mut futures = FuturesUnordered::new();

    for builder in builders.iter() {
        let params = params.clone();
        let builder_name = builder.name.clone();

        let future = async move {
            let result = match builder.send_bundle(params).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        Err(error.message)
                    } else {
                        if let Some(result) = response.result {
                            Ok(result)
                        } else {
                            // the builder (rsync / builder69) doesn't have any statistics API, therefore it won't have a BundleResult
                            if !builder.has_statistics_api {
                                Ok(BundleResult {
                                    bundle_hash: "                  Not available for this builder                  ".to_string(),
                                })
                            } else {
                                Err(format!("Result is None! {:?}", response.result))
                            }
                        }
                    }
                }
                Err(err) => Err(err.to_string()),
            };
            (builder_name, result)
        };

        futures.push(future);
    }

    let mut result_map = HashMap::new();

    while let Some((builder_name, result)) = futures.next().await {
        result_map.insert(builder_name, result);
    }

    result_map
}

pub async fn broadcast_end_of_block_bundle(
    params: EndOfBlockBundleParams,
    builders: Vec<&'static Builder>,
) -> HashMap<String, Result<BundleResult, String>> {
    let mut futures = FuturesUnordered::new();

    for builder in builders.iter() {
        if !builder.supports_eob() {
            continue;
        }

        let params = params.clone();
        let builder_name = builder.name.clone();

        let future = async move {
            let result = match builder.send_end_of_block_bundle(params).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        Err(error.message)
                    } else {
                        Ok(response.result.unwrap())
                    }
                }
                Err(err) => Err(err.to_string()),
            };
            (builder_name, result)
        };

        futures.push(future);
    }

    let mut result_map = HashMap::new();

    while let Some((builder_name, result)) = futures.next().await {
        result_map.insert(builder_name, result);
    }

    result_map
}