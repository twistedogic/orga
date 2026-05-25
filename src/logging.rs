use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;

pub struct Logger {
    file: Mutex<Option<File>>,
    debug: bool,
}

impl Logger {
    pub fn new(path: &Path, debug: bool) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok();
        Self {
            file: Mutex::new(file),
            debug,
        }
    }

    pub fn info(&self, msg: &str) {
        self.write_entry("INFO ", msg);
    }

    pub fn error(&self, msg: &str) {
        self.write_entry("ERROR", msg);
    }

    pub fn warn(&self, msg: &str) {
        self.write_entry("WARN ", msg);
    }

    pub fn debug(&self, msg: &str) {
        if self.debug {
            self.write_entry("DEBUG", msg);
        }
    }

    fn write_entry(&self, level: &str, msg: &str) {
        let ts = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = writeln!(f, "{ts} {level} {msg}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn error_and_warn_written_to_file() {
        let f = NamedTempFile::new().unwrap();
        let logger = Logger::new(f.path(), false);
        logger.error("something broke");
        logger.warn("something sketchy");
        let contents = fs::read_to_string(f.path()).unwrap();
        assert!(contents.contains("ERROR something broke"));
        assert!(contents.contains("WARN  something sketchy"));
    }

    #[test]
    fn debug_suppressed_when_disabled() {
        let f = NamedTempFile::new().unwrap();
        let logger = Logger::new(f.path(), false);
        logger.debug("hidden message");
        let contents = fs::read_to_string(f.path()).unwrap();
        assert!(!contents.contains("hidden message"));
    }

    #[test]
    fn debug_written_when_enabled() {
        let f = NamedTempFile::new().unwrap();
        let logger = Logger::new(f.path(), true);
        logger.debug("visible message");
        let contents = fs::read_to_string(f.path()).unwrap();
        assert!(contents.contains("DEBUG visible message"));
    }

    #[test]
    fn no_panic_when_path_unwritable() {
        let logger = Logger::new(Path::new("/nonexistent/dir/orga.log"), false);
        logger.error("should not panic");
        logger.warn("should not panic");
        logger.debug("should not panic");
    }
}
