# tracing-rolling-file-inc
[![Latest Version](https://img.shields.io/crates/v/tracing-rolling-file-inc.svg)](https://crates.io/crates/tracing-rolling-file-inc)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/tracing-rolling-file-inc)
![minimum rustc: 1.42](https://img.shields.io/badge/minimum%20rustc-1.42-yellowgreen?logo=rust&style=flat-square)

A rolling file appender with customizable rolling conditions,optimized the output method of file names to make them more scientific.
based on [tracing-rolling-file](https://github.com/cavivie/tracing-rolling-file).


This is useful to combine with the [tracing](https://crates.io/crates/tracing) crate and
[tracing_appender::non_blocking::NonBlocking](https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/index.html) -- use it
as an alternative to [tracing_appender::rolling::RollingFileAppender](https://docs.rs/tracing-appender/latest/tracing_appender/rolling/struct.RollingFileAppender.html).

## Examples

```rust
use tracing_rolling_file_inc::*;
let file_appender =
    RollingFileAppenderBase::new("./logs", "log", RollingConditionBase::new()
        .max_size(1024)
        .daily(), 50)?;
```
