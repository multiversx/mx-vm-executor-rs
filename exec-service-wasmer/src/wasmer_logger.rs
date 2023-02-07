use chrono::Local;

use log::{info, Level, LevelFilter, Metadata, Record};

struct WasmerLogger;

use std::sync::Once;

static INIT: Once = Once::new();

impl log::Log for WasmerLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{:<5}[{}] [{}]\t{}",
                record.level(),
                Local::now().format("%Y-%m-%d %H:%M:%S:%3f"),
                record
                    .file()
                    .unwrap()
                    .rsplit_terminator('/')
                    .next()
                    .unwrap(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init(log_level: LevelFilter) {
    INIT.call_once(|| {
        log::set_boxed_logger(Box::new(WasmerLogger))
            .map(|()| {
                log::set_max_level(log_level);
                info!("Initializing WasmerLogger with {log_level} ...");
            })
            .unwrap();
    });
}

pub fn set_log_level(log_level: LevelFilter) {
    if INIT.is_completed() {
        log::set_max_level(log_level);
        info!("Setting log level to {log_level} ...");
    } else {
        init(log_level);
    }
}

pub fn u64_to_log_level(value: u64) -> Result<LevelFilter, &'static str> {
    match value {
        0 => Ok(LevelFilter::Off),
        1 => Ok(LevelFilter::Error),
        2 => Ok(LevelFilter::Warn),
        3 => Ok(LevelFilter::Info),
        4 => Ok(LevelFilter::Debug),
        5 => Ok(LevelFilter::Trace),
        _ => Err("Undefined log level"),
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_init_only_once() {
        init(LevelFilter::Off);
        init(LevelFilter::Error);
        init(LevelFilter::Warn);
        init(LevelFilter::Info);
        init(LevelFilter::Debug);
        init(LevelFilter::Trace);
        assert_eq!(log::max_level(), LevelFilter::Off);
    }

    #[test]
    fn test_u64_to_log_level() {
        let result = u64_to_log_level(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Off);

        let result = u64_to_log_level(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Error);

        let result = u64_to_log_level(2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Warn);

        let result = u64_to_log_level(3);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Info);

        let result = u64_to_log_level(4);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Debug);

        let result = u64_to_log_level(5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LevelFilter::Trace);

        let result = u64_to_log_level(6);
        assert!(result.is_err());
    }
}
