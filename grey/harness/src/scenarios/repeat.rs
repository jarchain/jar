//! Scenario: submit 5 more pixels after serial, testing pipeline continuity.

use std::time::{Duration, Instant};

use crate::poll::submit_and_verify_pixel;
use crate::rpc::RpcClient;
use crate::scenarios::ScenarioResult;

const SERVICE_ID: u32 = 2000;
const TIMEOUT: Duration = Duration::from_secs(180);

const PIXELS: [(u8, u8, u8, u8, u8); 5] = [
    (60, 10, 128, 0, 0),   // dark red
    (70, 20, 0, 128, 0),   // dark green
    (80, 30, 0, 0, 128),   // dark blue
    (90, 40, 128, 128, 0), // dark yellow
    (99, 50, 128, 0, 128), // dark magenta
];

pub async fn run(client: &RpcClient) -> ScenarioResult {
    let start = Instant::now();
    for (x, y, r, g, b) in PIXELS {
        if let Err(e) = submit_and_verify_pixel(client, SERVICE_ID, x, y, r, g, b, TIMEOUT).await {
            return ScenarioResult {
                name: "repeat",
                pass: false,
                duration: start.elapsed(),
                error: Some(e.to_string()),
            };
        }
    }
    ScenarioResult {
        name: "repeat",
        pass: true,
        duration: start.elapsed(),
        error: None,
    }
}
