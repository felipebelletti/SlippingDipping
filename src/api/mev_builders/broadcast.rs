use colored::Colorize;

use crate::printlnt;

use super::{types::{EndOfBlockBundleParams, SendBundleParams}, BUILDERS};

pub fn broadcast_bundle(params: SendBundleParams) {
    for builder in BUILDERS.iter() {
        let params = params.clone();

        tokio::spawn(async move {
            match builder.send_bundle(params).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        printlnt!(
                            "{}",
                            format!(
                                "Normal Bundle Error | Reason: {} | Builder: {}",
                                &error.message, &builder.name
                            )
                            .red()
                        );
                        return;
                    }
                    printlnt!(
                        "{}",
                        format!(
                            "Normal Bundle Sent | Bundle Hash: {} | Builder: {}",
                            response.result.unwrap().bundle_hash,
                            builder.name
                        )
                        .yellow()
                    )
                }
                Err(err) => {
                    printlnt!(
                        "{}",
                        format!(
                            "Normal Bundle Broadcast Error | Error: {} | Builder: {}",
                            err, builder.name
                        )
                        .red()
                    )
                }
            };
        });
    }
}

pub fn broadcast_end_of_block_bundle(params: EndOfBlockBundleParams) {
    for builder in BUILDERS.iter() {
        let params = params.clone();

        tokio::spawn(async move {
            if !builder.supports_eob() {
                return;
            }

            match builder.send_end_of_block_bundle(params).await {
                Ok(response) => {
                    if let Some(error) = response.error {
                        printlnt!(
                            "{}",
                            format!(
                                "EoB Bundle Error | Reason: {} | Builder: {}",
                                &error.message, &builder.name
                            )
                            .red()
                        );
                        return;
                    }
                    printlnt!(
                        "{}",
                        format!(
                            "EoB Bundle Sent | Bundle Hash: {} | Builder: {}",
                            response.result.unwrap().bundle_hash,
                            builder.name
                        )
                        .yellow()
                    )
                }
                Err(err) => {
                    printlnt!(
                        "{}",
                        format!(
                            "EoB Broadcast Error | Error: {} | Builder: {}",
                            err, builder.name
                        )
                        .red()
                    )
                }
            };
        });
    }
}
