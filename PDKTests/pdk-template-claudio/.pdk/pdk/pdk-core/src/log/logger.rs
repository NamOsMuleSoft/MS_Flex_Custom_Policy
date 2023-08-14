// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::HostTrait;
use classy::proxy_wasm::types::LogLevel;

use crate::host::property::PropertyAccessor;
use crate::log::log_metadata::LogMetadata;

struct Logger;

static LOGGER: Logger = Logger;
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn set_log_level(level: LogLevel) {
    if !INITIALIZED.load(Ordering::Relaxed) {
        let _ = log::set_logger(&LOGGER);
        panic::set_hook(Box::new(|panic_info| {
            crate::Host.log(LogLevel::Critical, &panic_info.to_string());
        }));
        INITIALIZED.store(true, Ordering::Relaxed);
    }
    log::set_max_level(to_log_lib_level(level));
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let metadata = LogMetadata::from(<dyn PropertyAccessor>::default());
        let message = format!("{} {}", metadata, &record.args());
        let level = to_proxy_level(record.level());
        crate::Host.log(level, &message);
    }

    fn flush(&self) {}
}

pub fn to_log_lib_level(level: LogLevel) -> log::LevelFilter {
    match level {
        LogLevel::Trace => log::LevelFilter::Trace,
        LogLevel::Debug => log::LevelFilter::Debug,
        LogLevel::Info => log::LevelFilter::Info,
        LogLevel::Warn => log::LevelFilter::Warn,
        LogLevel::Error => log::LevelFilter::Error,
        LogLevel::Critical => log::LevelFilter::Off,
    }
}

pub fn to_proxy_level(level: log::Level) -> LogLevel {
    match level {
        log::Level::Trace => LogLevel::Trace,
        log::Level::Debug => LogLevel::Debug,
        log::Level::Info => LogLevel::Info,
        log::Level::Warn => LogLevel::Warn,
        log::Level::Error => LogLevel::Error,
    }
}
