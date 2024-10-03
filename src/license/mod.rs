use std::{fs, error::Error};

use md5;
use get_if_addrs::{get_if_addrs, IfAddr};
use hex::ToHex;
use serde_json::json;
use tokio::task::JoinHandle;

const LICENSE_URL_ROOT: &str = "http://168.75.88.187:25565/";

pub async fn check_license() -> Result<bool, Box<dyn Error>> {
    let ifs = get_if_addrs().expect("Error 100");
    let local_ip = ifs
        .into_iter()
        .find(|iface| {
            !iface.is_loopback()
                && match iface.addr {
                    IfAddr::V4(_) => true,
                    IfAddr::V6(_) => false,
                }
        })
        .ok_or("Err 201")?
        .addr
        .ip()
        .to_string();

    let machine_id_raw = fs::read_to_string("/etc/machine-id")?;
    let machine_id = machine_id_raw.trim();

    let hashed_license = md5::compute(format!(
        "{}x{}x{}",
        local_ip,
        machine_id,
        whoami::username()
    ));
    let hex_encoded_license = hex::encode(hashed_license.to_vec());

    let response = reqwest::Client::new()
        .post(format!("{}{}", LICENSE_URL_ROOT, "verify"))
        .body(json!({"ASFokfds": hex_encoded_license}).to_string())
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let is_valid = response.json::<bool>().await?;

    Ok(is_valid)
}

pub fn send_telemetry_message(message: String) {
    tokio::spawn(async move {
        let _ = reqwest::Client::new()
            .post(format!("{}{}", LICENSE_URL_ROOT, "telemetry"))
            .body(json!({"message": message}).to_string())
            .header("Content-Type", "application/json")
            .send()
            .await;
    });
}
