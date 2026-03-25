//! Integration test scenarios.

pub mod liveness;
pub mod repeat;
pub mod serial;

use std::time::Duration;

/// Result of a single scenario run.
#[allow(dead_code)]
pub struct ScenarioResult {
    pub name: &'static str,
    pub pass: bool,
    pub duration: Duration,
    pub error: Option<String>,
}
