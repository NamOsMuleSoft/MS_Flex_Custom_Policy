// Copyright 2023 Salesforce, Inc. All rights reserved.
use classy::proxy_wasm::types::LogLevel;

mod log_metadata;
pub mod logger;

use crate::host::property::PropertyAccessor;
use crate::policy_context::metadata::PolicyMetadata;
pub use log::{debug, error, info, trace, warn};

pub const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Info;

const LOG_LEVELS: [(&str, LogLevel); 7] = [
    ("trace", LogLevel::Trace),
    ("debug", LogLevel::Debug),
    ("info", LogLevel::Info),
    ("warn", LogLevel::Warn),
    ("warning", LogLevel::Warn),
    ("error", LogLevel::Error),
    ("critical", LogLevel::Critical),
];

pub fn configure_logger() {
    let metadata = PolicyMetadata::from(<dyn PropertyAccessor>::default());
    logger::set_log_level(get_min_level_from_context(&metadata));
}

fn get_min_level_from_context(metadata: &PolicyMetadata) -> LogLevel {
    metadata
        .policy_config()
        .and_then(|config| config.logging())
        .map(|logging| by_name(logging.level()))
        .unwrap_or(DEFAULT_LOG_LEVEL)
}

fn by_name(level: &str) -> LogLevel {
    LOG_LEVELS
        .iter()
        .find(|(level_name, _)| level_name.eq_ignore_ascii_case(level))
        .map_or_else(|| DEFAULT_LOG_LEVEL, |(_, log_level)| *log_level)
}

#[cfg(test)]
mod tests {
    use classy::proxy_wasm::types::LogLevel;
    use test_case::test_case;

    use crate::logger::by_name;

    #[test_case("trace", LogLevel::Trace; "when level is trace")]
    #[test_case("debug", LogLevel::Debug; "when level is debug")]
    #[test_case("info", LogLevel::Info; "when level is info")]
    #[test_case("warn", LogLevel::Warn; "when level is warn")]
    #[test_case("warning", LogLevel::Warn; "when level is warning")]
    #[test_case("error", LogLevel::Error; "when level is error")]
    #[test_case("critical", LogLevel::Critical; "when level is critical")]
    #[test_case("other", LogLevel::Info; "when level is other")]
    #[test_case("", LogLevel::Info; "when level is empty")]
    #[test_case("TRACE", LogLevel::Trace; "when level is trace, with no-lowercased")]
    fn test_from_str(input: &str, expected: LogLevel) {
        assert_eq!(by_name(input), expected);
    }
}
