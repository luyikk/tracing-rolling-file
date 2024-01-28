# tracing-rolling-file

![license: MIT or Apache-2.0](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-red?style=flat-square)
![minimum rustc: 1.42](https://img.shields.io/badge/minimum%20rustc-1.42-yellowgreen?logo=rust&style=flat-square)

A rolling file appender with customizable rolling conditions,optimized the output method of file names to make them more scientific.
based on [rolling-file](https://github.com/cavivie/tracing-rolling-file).


This is useful to combine with the [tracing](https://crates.io/crates/tracing) crate and
[tracing_appender::non_blocking::NonBlocking](https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/index.html) -- use it
as an alternative to [tracing_appender::rolling::RollingFileAppender](https://docs.rs/tracing-appender/latest/tracing_appender/rolling/struct.RollingFileAppender.html).

## Examples

```rust
use tracing_rolling_file_inc::*;
let file_appender =
    RollingFileAppenderBase::new("./logs", "foo", RollingConditionBase::new()
        .max_size(1024)
        .daily(), 50)?;
```

## License

Dual-licensed under the terms of either the MIT license or the Apache 2.0 license.
