//! Polling utilities: wait_until, RPC readiness, service readiness, pixel verification.

use std::time::{Duration, Instant};

use tracing::info;

use crate::pixel;
use crate::rpc::RpcClient;

#[derive(Debug, thiserror::Error)]
#[error("timed out waiting for {label} ({timeout:?})")]
pub struct TimeoutError {
    label: String,
    timeout: Duration,
}

/// Poll `predicate` every `interval` until it returns `true`, or fail after `timeout`.
pub async fn wait_until<F, Fut>(
    predicate: F,
    interval: Duration,
    timeout: Duration,
    label: &str,
) -> Result<(), TimeoutError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate().await {
            return Ok(());
        }
        tokio::time::sleep(interval).await;
    }
    Err(TimeoutError {
        label: label.to_string(),
        timeout,
    })
}

/// Wait for the RPC endpoint to respond.
pub async fn wait_for_rpc(client: &RpcClient, timeout: Duration) -> Result<(), TimeoutError> {
    wait_until(
        || async { client.get_status().await.is_ok() },
        Duration::from_secs(1),
        timeout,
        "RPC ready",
    )
    .await
}

/// Wait for a service to have a non-null code_hash.
pub async fn wait_for_service(
    client: &RpcClient,
    service_id: u32,
    timeout: Duration,
) -> Result<(), TimeoutError> {
    wait_until(
        || async {
            client
                .get_context(service_id)
                .await
                .map(|ctx| ctx.code_hash.is_some())
                .unwrap_or(false)
        },
        Duration::from_secs(2),
        timeout,
        &format!("service {service_id} ready"),
    )
    .await
}

/// Check if a pixel at (x,y) with color (r,g,b) is written in storage.
pub async fn check_pixel(
    client: &RpcClient,
    service_id: u32,
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
) -> bool {
    let Ok(storage) = client.read_storage(service_id, "00").await else {
        return false;
    };
    let Some(value) = &storage.value else {
        return false;
    };
    let offset = (y as usize * 100 + x as usize) * 3 * 2; // hex offset
    if offset + 6 > value.len() {
        return false;
    }
    let expected = format!("{r:02x}{g:02x}{b:02x}");
    &value[offset..offset + 6] == expected
}

/// Submit a pixel work package and wait for it to appear in storage.
pub async fn submit_and_verify_pixel(
    client: &RpcClient,
    service_id: u32,
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = client.get_context(service_id).await?;
    assert!(ctx.code_hash.is_some(), "service code_hash must be non-null");

    let wp_bytes = pixel::build_pixel_work_package(service_id, &ctx, x, y, r, g, b)?;
    let data_hex = hex::encode(&wp_bytes);
    let result = client.submit_work_package(&data_hex).await?;
    let color_hex = format!("#{r:02x}{g:02x}{b:02x}");
    info!(
        "submitted ({x},{y}) {color_hex} hash={}...",
        &result.hash[..16.min(result.hash.len())]
    );

    wait_until(
        || async { check_pixel(client, service_id, x, y, r, g, b).await },
        Duration::from_secs(2),
        timeout,
        &format!("pixel ({x},{y}) {color_hex}"),
    )
    .await?;

    let storage = client.read_storage(service_id, "00").await?;
    info!("({x},{y}) {color_hex} ok (slot {})", storage.slot);
    Ok(())
}
