//! A rolling file appender with customizable rolling conditions.
//! Includes built-in support for rolling conditions on date/time
//! (daily, hourly, every minute) and/or size.
//!
//! Follows a Debian-style naming convention for logfiles,
//! using basename, basename.1, ..., basename.N where N is
//! the maximum number of allowed historical logfiles.
//!
//! This is useful to combine with the tracing crate and
//! tracing_appender::non_blocking::NonBlocking -- use it
//! as an alternative to tracing_appender::rolling::RollingFileAppender.
//!
//! # Examples
//!
//! ```rust
//! # fn docs() {
//! # use tracing_rolling_file_inc::*;
//! let file_appender = RollingFileAppenderBase::new(
//!     "./logs",
//!     "foo",
//!     RollingConditionBase::new().daily(),
//!     9
//! ).unwrap();
//! # }
//! ```

use chrono::prelude::*;
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    convert::TryFrom,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::Path,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RollingFileError {
    #[error("io error:")]
    IOError(#[from] io::Error),
    #[error("io error:")]
    RegexError(#[from] regex::Error),
}

/// Determines when a file should be "rolled over".
pub trait RollingCondition {
    /// Determine and return whether or not the file should be rolled over.
    fn should_rollover(&mut self, now: &DateTime<Local>, current_filesize: u64) -> bool;
}

/// Determines how often a file should be rolled over
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RollingFrequency {
    EveryDay,
    EveryHour,
    EveryMinute,
}

impl RollingFrequency {
    /// Calculates a datetime that will be different if data should be in
    /// different files.
    pub fn equivalent_datetime(&self, dt: &DateTime<Local>) -> DateTime<Local> {
        let (year, month, day) = (dt.year(), dt.month(), dt.day());
        let (hour, min, sec) = match self {
            RollingFrequency::EveryDay => (0, 0, 0),
            RollingFrequency::EveryHour => (dt.hour(), 0, 0),
            RollingFrequency::EveryMinute => (dt.hour(), dt.minute(), 0),
        };
        Local.with_ymd_and_hms(year, month, day, hour, min, sec).unwrap()
    }
}

/// Writes data to a file, and "rolls over" to preserve older data in
/// a separate set of files. Old files have a Debian-style naming scheme
/// where we have base_filename, base_filename.1, ..., base_filename.N
/// where N is the maximum number of rollover files to keep.
#[derive(Debug)]
pub struct RollingFileAppender<RC>
where
    RC: RollingCondition,
{
    condition: RC,
    directory: PathBuf,
    suffix: String,
    file_index: AtomicUsize,
    max_file_count: usize,
    current_filesize: u64,
    writer_opt: Option<BufWriter<File>>,
}

impl<RC> RollingFileAppender<RC>
where
    RC: RollingCondition,
{
    /// Creates a new rolling file appender with the given condition.
    /// The filename parent path must already exist.
    pub fn new(
        directory: impl AsRef<Path>,
        suffix: &str,
        condition: RC,
        max_file_count: usize,
    ) -> Result<RollingFileAppender<RC>, RollingFileError> {
        let directory = directory.as_ref().to_owned();

        let (file_index, current_filesize) = {
            if !directory.exists() {
                fs::create_dir_all(directory.as_path())?;
                (AtomicUsize::new(1), 0)
            } else {
                let dirs = fs::read_dir(directory.as_path())?;
                let mut current_indexes = vec![];
                let re = Regex::new(r"\d+")?;
                for dir in dirs {
                    let dir = dir?;
                    if dir.file_type()?.is_file() {
                        if let Some(filename) = dir.file_name().to_str() {
                            if let Some(cp) = re.captures(filename) {
                                if let Ok(index) = usize::from_str(&cp[0]) {
                                    current_indexes.push(index);
                                }
                            }
                        }
                    }
                }

                if !current_indexes.is_empty() {
                    current_indexes.sort();
                    current_indexes.reverse();

                    let current_filesize = {
                        let has_curr_log = directory.join(format!("{}.current.log", suffix));
                        if has_curr_log.exists() {
                            fs::metadata(has_curr_log)?.len()
                        } else {
                            0
                        }
                    };

                    let max_index = current_indexes[0];
                    (AtomicUsize::new(max_index + 1), current_filesize)
                } else {
                    (AtomicUsize::new(1), 0)
                }
            }
        };

        let mut appender = RollingFileAppender {
            condition,
            directory,
            suffix: suffix.to_string(),
            file_index,
            max_file_count,
            current_filesize,
            writer_opt: None,
        };
        // Fail if we can't open the file initially...
        appender.open_writer_if_needed()?;
        Ok(appender)
    }

    /// Determines the final filename, where n==0 indicates the current file
    fn filename_for(&self, n: usize) -> PathBuf {
        let f = self.suffix.clone();
        if n > 0 {
            self.directory.join(format!("{}.{}.log", f, n))
        } else {
            self.directory.join(format!("{}.current.log", f))
        }
    }

    /// Rotates old files to make room for a new one.
    /// This may result in the deletion of the oldest file
    fn rotate_files(&mut self) -> io::Result<()> {
        let remove_index = self.file_index.load(Ordering::Acquire) as i64 - self.max_file_count as i64;
        if remove_index > 0 {
            let _ = fs::remove_file(self.filename_for(remove_index as usize));
        }

        let to_index = self.file_index.fetch_add(1, Ordering::Acquire);
        let mut r = Ok(());
        if let Err(e) = fs::rename(self.filename_for(0), self.filename_for(to_index)).or_else(|e| match e.kind() {
            io::ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        }) {
            // capture the error, but continue the loop,
            // to maximize ability to rename everything
            r = Err(e);
        }

        r
    }

    /// Forces a rollover to happen immediately.
    pub fn rollover(&mut self) -> io::Result<()> {
        // Before closing, make sure all data is flushed successfully.
        self.flush()?;
        // We must close the current file before rotating files
        self.writer_opt.take();
        self.current_filesize = 0;
        self.rotate_files()?;
        self.open_writer_if_needed()
    }

    /// Opens a writer for the current file.
    fn open_writer_if_needed(&mut self) -> io::Result<()> {
        if self.writer_opt.is_none() {
            let path = self.filename_for(0);
            let path = Path::new(&path);
            let mut open_options = OpenOptions::new();
            open_options.append(true).create(true);
            let new_file = match open_options.open(path) {
                Ok(new_file) => new_file,
                Err(err) => {
                    let Some(parent) = path.parent() else {
                        return Err(err);
                    };
                    fs::create_dir_all(parent)?;
                    open_options.open(path)?
                },
            };
            self.writer_opt = Some(BufWriter::new(new_file));
            self.current_filesize = path.metadata().map_or(0, |m| m.len());
        }
        Ok(())
    }

    /// Writes data using the given datetime to calculate the rolling condition
    pub fn write_with_datetime(&mut self, buf: &[u8], now: &DateTime<Local>) -> io::Result<usize> {
        if self.condition.should_rollover(now, self.current_filesize) {
            if let Err(e) = self.rollover() {
                // If we can't rollover, just try to continue writing anyway
                // (better than missing data).
                // This will likely used to implement logging, so
                // avoid using log::warn and log to stderr directly
                eprintln!("WARNING: Failed to rotate logfile {}: {}", self.suffix, e);
            }
        }
        self.open_writer_if_needed()?;
        if let Some(writer) = self.writer_opt.as_mut() {
            let buf_len = buf.len();
            writer.write_all(buf).map(|_| {
                self.current_filesize += u64::try_from(buf_len).unwrap_or(u64::MAX);
                buf_len
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected condition: writer is missing",
            ))
        }
    }
}

impl<RC> io::Write for RollingFileAppender<RC>
where
    RC: RollingCondition,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = Local::now();
        self.write_with_datetime(buf, &now)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(writer) = self.writer_opt.as_mut() {
            writer.flush()?;
        }
        Ok(())
    }
}

pub mod base;
pub use base::*;
