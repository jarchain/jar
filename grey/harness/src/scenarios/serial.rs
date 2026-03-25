//! Scenario: submit 5 pixels sequentially, verify each one.

use std::time::{Duration, Instant};

use crate::poll::submit_and_verify_pixel;
use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

const SERVICE_ID: u32 = 2000;
const TIMEOUT: Duration = Duration::from_secs(120);

const PIXELS: [(u8, u8, u8, u8, u8); 5] = [
    (10, 10, 255, 0, 0),   // red
    (20, 20, 0, 255, 0),   // green
    (30, 30, 0, 0, 255),   // blue
    (40, 40, 255, 255, 0), // yellow
    (50, 50, 255, 0, 255), // magenta
];

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();
    for (x, y, r, g, b) in PIXELS {
        if let Err(e) = submit_and_verify_pixel(client, SERVICE_ID, x, y, r, g, b, TIMEOUT).await {
            return ScenarioResult {
                name: "serial",
                pass: false,
                duration: start.elapsed(),
                error: Some(e.to_string()),
            };
        }
    }
    ScenarioResult {
        name: "serial",
        pass: true,
        duration: start.elapsed(),
        error: None,
    }
}
