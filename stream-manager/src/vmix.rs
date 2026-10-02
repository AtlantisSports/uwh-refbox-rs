//! Starts and stops a vMix streaming destination through vMix's Web Controller API
//! (`http://<address>/api/?Function=StartStreaming&Value=<n>`, where n = destination − 1).

use crate::BoxError;
use std::time::Duration;

fn client() -> Result<reqwest::Client, BoxError> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?)
}

async fn call(address: &str, function: &str, destination: u8) -> Result<(), BoxError> {
    let value = destination.saturating_sub(1).to_string();
    let url = format!("http://{address}/api/");
    let response = client()?
        .get(&url)
        .query(&[("Function", function), ("Value", value.as_str())])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach vMix at {address}: {e}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "vMix at {address} refused {function} for destination {destination} ({})",
            response.status()
        )
        .into())
    }
}

/// Starts sending on streaming destination `destination` (1, 2 or 3, as numbered in vMix).
pub async fn start_destination(address: &str, destination: u8) -> Result<(), BoxError> {
    call(address, "StartStreaming", destination).await
}

pub async fn stop_destination(address: &str, destination: u8) -> Result<(), BoxError> {
    call(address, "StopStreaming", destination).await
}

/// True if vMix's Web Controller answers at `address`.
pub async fn is_reachable(address: &str) -> bool {
    match client() {
        Ok(c) => c
            .get(format!("http://{address}/api/"))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success()),
        Err(_) => false,
    }
}
