use chrono::Local;

use log::{info, Level, LevelFilter, Metadata, Record};

struct WasmerLogger;

impl log::Log for WasmerLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{}[{}] [{}]\t{}",
                record.level(),
                Local::now().format("%Y-%m-%d %H:%M:%S:%3f"),
                record
                    .file()
                    .unwrap()
                    .rsplit_terminator("/")
                    .next()
                    .unwrap(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init(log_level: LevelFilter) -> bool {
    let result =
        log::set_boxed_logger(Box::new(WasmerLogger)).map(|()| log::set_max_level(log_level));

    match result {
        Ok(_) => {
            let log_level = log::max_level();
            info!("Initializing WasmerLogger with {log_level} ...");
            true
        }
        Err(_) => {
            info!("WasmerLogger already initialized");
            false
        }
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
    fn test_already_initialized() {
        let result = init(LevelFilter::Off);
        assert!(result);
        let result = init(LevelFilter::Off);
        assert!(!result);
    }

    #[test]
    fn test_set_max_log_level() {
        init(LevelFilter::Off);
        assert_eq!(log::max_level(), LevelFilter::Off);

        log::set_max_level(LevelFilter::Debug);
        assert_eq!(log::max_level(), LevelFilter::Debug);
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
