pub mod dipper;
pub mod erc20;

use core::error;
use std::sync::Arc;
use std::time::Duration;

use alloy::eips::eip2718::Encodable2718;
use alloy::network::{NetworkWallet, TransactionBuilder};
use alloy::primitives::utils::parse_ether;
use alloy::rpc::types::TransactionReceipt;
use alloy::signers::local::PrivateKeySigner;
use alloy::{consensus::TxEnvelope, providers::Provider};
use colored::Colorize;
use regex::Regex;
use revm::primitives::{FixedBytes, U256};
use tokio::time::sleep;
use unicode_width::UnicodeWidthStr;

use crate::config::wallet::types::Wallet;
use crate::printlnt;
use crate::{config::general::GLOBAL_CONFIG, Dipper};

fn strip_ansi_codes(s: &str) -> String {
    // Expressão regular para remover códigos ANSI
    let re = Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").to_string()
}

fn pad_to_width(s: &str, width: usize) -> String {
    let stripped = strip_ansi_codes(s);
    let display_width = UnicodeWidthStr::width(stripped.as_str());
    let padding_needed = width.saturating_sub(display_width);
    let padded = format!("{}{}", s, " ".repeat(padding_needed));
    padded
}

pub fn print_pretty_dashboard(header_text: &str, rows: Vec<String>) {
    // Remove códigos ANSI do texto do cabeçalho
    let stripped_header_text = strip_ansi_codes(header_text);
    let header_text_width = UnicodeWidthStr::width(stripped_header_text.as_str());

    // Calcula a largura máxima das linhas de conteúdo sem códigos ANSI
    let max_content_width = rows
        .iter()
        .map(|row| {
            let stripped_row = strip_ansi_codes(row);
            UnicodeWidthStr::width(stripped_row.as_str())
        })
        .max()
        .unwrap_or(0);

    // Adiciona 2 ao conteúdo para considerar os espaços entre o texto e as bordas verticais
    let content_width_with_spaces = max_content_width + 2; // +2 para um espaço em cada lado
    let header_width_with_spaces = header_text_width + 2; // +2 se quiser espaços no cabeçalho

    // A largura total é o maior valor entre o conteúdo e o título, com ajustes
    let total_width = content_width_with_spaces
        .max(header_width_with_spaces)
        .max(30); // Define uma largura mínima para evitar caixas muito estreitas

    // Cria o cabeçalho e rodapé com base na largura total
    let header_padding_total = total_width - header_text_width;
    let header_padding_left = header_padding_total / 2;
    let header_padding_right = header_padding_total - header_padding_left;

    let header = format!(
        "╭{}{}{}╮",
        "─".repeat(header_padding_left),
        header_text,
        "─".repeat(header_padding_right)
    );
    let footer = format!("╰{}╯", "─".repeat(total_width));

    // Exibe o cabeçalho
    println!("{}", header.bold().green());

    // Exibe cada linha de conteúdo com padding adequado
    let green_vertical_row_char = "│".green();
    for row in rows {
        // Padroniza a linha para a largura total menos 2 (espaços laterais)
        let padded_row = pad_to_width(&row, total_width - 2);
        println!(
            "{green_vertical_row_char} {} {green_vertical_row_char}",
            padded_row
        ); // Adiciona um espaço antes e depois do conteúdo
    }

    // Exibe o rodapé
    println!("{}", footer.bold().green());
}

pub fn tx_envelope_to_raw_tx(envelope: TxEnvelope) -> Vec<u8> {
    let mut encoded_tx = vec![];
    envelope.encode_2718(&mut encoded_tx);
    return encoded_tx;
}

pub async fn get_raw_bribe_tx<M: Provider>(
    client: Arc<M>,
    signer_wallet: Wallet,
    nonce: u64,
    bribe_amount: f64,
    target_block_number: U256,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let dipper = Dipper::new(GLOBAL_CONFIG.general.dipper_contract, client.clone());

    let estimate_eip1559_fees = client
        .estimate_eip1559_fees(None)
        .await
        .map_err(|err| format!("estimate_eip1559_fees err: {err}"))?;

    let tx = dipper
        .paybribe_81014001426369(target_block_number)
        .value(parse_ether(bribe_amount.to_string().as_ref()).unwrap())
        .from(signer_wallet.address)
        .nonce(nonce)
        .gas(40804)
        .max_priority_fee_per_gas(estimate_eip1559_fees.max_priority_fee_per_gas)
        .max_fee_per_gas(estimate_eip1559_fees.max_fee_per_gas);

    let raw_tx = tx_envelope_to_raw_tx(
        tx.into_transaction_request()
            .build(&signer_wallet.signer)
            .await
            .unwrap(),
    );

    Ok(raw_tx)
}

pub async fn get_tx_receipt<M: Provider + 'static>(
    client: Arc<M>,
    hash: FixedBytes<32>,
    max_attempts: usize,
    delay_between_requests: f64,
    debug: bool
) -> Option<TransactionReceipt> {
    for attempt in 1..=max_attempts {
        match client.get_transaction_receipt(hash).await {
            Ok(Some(receipt)) => {
                return Some(receipt);
            }
            Ok(None) => {
                if debug {
                    printlnt!(
                        "Attempt {}/{}: Transaction receipt not yet available for tx: {}",
                        attempt, max_attempts, hash
                    );
                }
            }
            Err(err) => {
                printlnt!("Error getting transaction receipt: {err}");
                return None;
            }
        }

        if attempt < max_attempts {
            sleep(Duration::from_secs_f64(delay_between_requests)).await;
        }
    }

    if debug {
        printlnt!(
            "Exceeded maximum attempts ({}) to get transaction receipt for tx: {}",
            max_attempts, hash
        );
    }

    None
}
