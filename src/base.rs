//! Implements a rolling condition based on a certain frequency
//! and/or a size limit. The default condition is to rotate daily.
//!
//! # Examples
//!
//! ```rust
//! use tracing_rolling_file_inc::*;
//! let c = RollingConditionBase::new().daily();
//! let c = RollingConditionBase::new().hourly().max_size(1024 * 1024);
//! ```

use crate::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct RollingConditionBase {
    last_write_opt: Option<DateTime<Local>>,
    frequency_opt: Option<RollingFrequency>,
    max_size_opt: Option<u64>,
}

impl RollingConditionBase {
    /// Constructs a new struct that does not yet have any condition set.
    pub fn new() -> RollingConditionBase {
        RollingConditionBase {
            last_write_opt: None,
            frequency_opt: None,
            max_size_opt: None,
        }
    }

    /// Sets a condition to rollover on the given frequency
    pub fn frequency(mut self, x: RollingFrequency) -> RollingConditionBase {
        self.frequency_opt = Some(x);
        self
    }

    /// Sets a condition to rollover when the date changes
    pub fn daily(mut self) -> RollingConditionBase {
        self.frequency_opt = Some(RollingFrequency::EveryDay);
        self
    }

    /// Sets a condition to rollover when the date or hour changes
    pub fn hourly(mut self) -> RollingConditionBase {
        self.frequency_opt = Some(RollingFrequency::EveryHour);
        self
    }

    /// Sets a condition to rollover when the date or minute changes
    pub fn minutely(mut self) -> RollingConditionBase {
        self.frequency_opt = Some(RollingFrequency::EveryMinute);
        self
    }

    /// Sets a condition to rollover when a certain size is reached
    pub fn max_size(mut self, x: u64) -> RollingConditionBase {
        self.max_size_opt = Some(x);
        self
    }
}

impl Default for RollingConditionBase {
    fn default() -> Self {
        RollingConditionBase::new().frequency(RollingFrequency::EveryDay)
    }
}

impl RollingCondition for RollingConditionBase {
    fn should_rollover(&mut self, now: &DateTime<Local>, current_filesize: u64) -> bool {
        let mut rollover = false;
        if let Some(frequency) = self.frequency_opt.as_ref() {
            if let Some(last_write) = self.last_write_opt.as_ref() {
                if frequency.equivalent_datetime(now) != frequency.equivalent_datetime(last_write) {
                    rollover = true;
                }
            }
        }
        if let Some(max_size) = self.max_size_opt.as_ref() {
            if current_filesize >= *max_size {
                rollover = true;
            }
        }
        self.last_write_opt = Some(*now);
        rollover
    }
}

/// A rolling file appender with a rolling condition based on date/time or size.
pub type RollingFileAppenderBase = RollingFileAppender<RollingConditionBase>;

// LCOV_EXCL_START
#[cfg(test)]
mod test {
    use super::*;

    struct Context {
        _tempdir: tempfile::TempDir,
        rolling: RollingFileAppenderBase,
    }

    impl Context {
        fn verify_contains(&mut self, needle: &str, n: usize) {
            self.rolling.flush().unwrap();
            let p = self.rolling.filename_for(n);
            let haystack = fs::read_to_string(&p).unwrap();
            if !haystack.contains(needle) {
                panic!("file {:?} did not contain expected contents {}", p, needle);
            }
        }
    }

    fn build_context(condition: RollingConditionBase, max_files: usize) -> Context {
        let tempdir = tempfile::tempdir().unwrap();

        let rolling = RollingFileAppenderBase::new(tempdir.as_ref(), "test", condition, max_files).unwrap();
        Context {
            _tempdir: tempdir,
            rolling,
        }
    }

    #[test]
    fn frequency_every_day() {
        let mut c = build_context(RollingConditionBase::new().daily(), 9);
        c.rolling
            .write_with_datetime(b"Line 1\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 2\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 3\n", &Local.with_ymd_and_hms(2021, 3, 31, 1, 4, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 4\n", &Local.with_ymd_and_hms(2021, 5, 31, 1, 4, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 5\n", &Local.with_ymd_and_hms(2022, 5, 31, 1, 4, 0).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(4)).exists());
        c.verify_contains("Line 1", 1);
        c.verify_contains("Line 2", 1);
        c.verify_contains("Line 3", 2);
        c.verify_contains("Line 4", 3);
        c.verify_contains("Line 5", 0);
    }

    #[test]
    fn frequency_every_day_limited_files() {
        let mut c = build_context(RollingConditionBase::new().daily(), 2);
        c.rolling
            .write_with_datetime(b"Line 1\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 2\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 3\n", &Local.with_ymd_and_hms(2021, 3, 31, 1, 4, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 4\n", &Local.with_ymd_and_hms(2021, 5, 31, 1, 4, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 5\n", &Local.with_ymd_and_hms(2022, 5, 31, 1, 4, 0).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(1)).exists());
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(4)).exists());
        c.verify_contains("Line 3", 2);
        c.verify_contains("Line 4", 3);
        c.verify_contains("Line 5", 0);
    }

    #[test]
    fn frequency_every_hour() {
        let mut c = build_context(RollingConditionBase::new().hourly(), 9);
        c.rolling
            .write_with_datetime(b"Line 1\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 2\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 2).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 3\n", &Local.with_ymd_and_hms(2021, 3, 30, 2, 1, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 4\n", &Local.with_ymd_and_hms(2021, 3, 31, 2, 1, 0).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(3)).exists());
        c.verify_contains("Line 1", 1);
        c.verify_contains("Line 2", 1);
        c.verify_contains("Line 3", 2);
        c.verify_contains("Line 4", 0);
    }

    #[test]
    fn frequency_every_minute() {
        let mut c = build_context(RollingConditionBase::new().frequency(RollingFrequency::EveryMinute), 9);
        c.rolling
            .write_with_datetime(b"Line 1\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 2\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 3\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 4).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 4\n", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 5\n", &Local.with_ymd_and_hms(2021, 3, 30, 2, 3, 0).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"Line 6\n", &Local.with_ymd_and_hms(2022, 3, 30, 2, 3, 0).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(4)).exists());
        c.verify_contains("Line 1", 1);
        c.verify_contains("Line 2", 1);
        c.verify_contains("Line 3", 1);
        c.verify_contains("Line 4", 2);
        c.verify_contains("Line 5", 3);
        c.verify_contains("Line 6", 0);
    }

    #[test]
    fn max_size() {
        let mut c = build_context(RollingConditionBase::new().max_size(10), 9);
        c.rolling
            .write_with_datetime(b"12345", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"6789", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"0", &Local.with_ymd_and_hms(2021, 3, 30, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"abcdefghijkl", &Local.with_ymd_and_hms(2021, 3, 31, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"ZZZ", &Local.with_ymd_and_hms(2022, 3, 31, 1, 2, 3).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(3)).exists());
        c.verify_contains("1234567890", 1);
        c.verify_contains("abcdefghijkl", 2);
        c.verify_contains("ZZZ", 0);
    }

    #[test]
    fn max_size_existing() {
        let mut c = build_context(RollingConditionBase::new().max_size(10), 9);
        c.rolling
            .write_with_datetime(b"12345", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        // close the file and make sure that it can re-open it, and that it
        // resets the file size properly.
        c.rolling.writer_opt.take();
        c.rolling.current_filesize = 0;
        c.rolling
            .write_with_datetime(b"6789", &Local.with_ymd_and_hms(2021, 3, 30, 1, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"0", &Local.with_ymd_and_hms(2021, 3, 30, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"abcdefghijkl", &Local.with_ymd_and_hms(2021, 3, 31, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"ZZZ", &Local.with_ymd_and_hms(2022, 3, 31, 1, 2, 3).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(3)).exists());
        c.verify_contains("1234567890", 1);
        c.verify_contains("abcdefghijkl", 2);
        c.verify_contains("ZZZ", 0);
    }

    #[test]
    fn daily_and_max_size() {
        let mut c = build_context(RollingConditionBase::new().daily().max_size(10), 9);
        c.rolling
            .write_with_datetime(b"12345", &Local.with_ymd_and_hms(2021, 3, 30, 1, 2, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"6789", &Local.with_ymd_and_hms(2021, 3, 30, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"0", &Local.with_ymd_and_hms(2021, 3, 31, 2, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"abcdefghijkl", &Local.with_ymd_and_hms(2021, 3, 31, 3, 3, 3).unwrap())
            .unwrap();
        c.rolling
            .write_with_datetime(b"ZZZ", &Local.with_ymd_and_hms(2021, 3, 31, 4, 4, 4).unwrap())
            .unwrap();
        assert!(!AsRef::<Path>::as_ref(&c.rolling.filename_for(3)).exists());
        c.verify_contains("123456789", 1);
        c.verify_contains("0abcdefghijkl", 2);
        c.verify_contains("ZZZ", 0);
    }
}
// LCOV_EXCL_STOP
