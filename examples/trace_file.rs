use tracing::{instrument, span, Level};
use tracing_rolling_file_inc::{RollingConditionBase, RollingFileAppenderBase};
use tracing_subscriber::{fmt, layer::SubscriberExt};

fn main() -> anyhow::Result<()> {
    let file_appender =
        RollingFileAppenderBase::new("./logs", "log", RollingConditionBase::new().max_size(1024).daily(), 50)?;

    let filters = "TRACE";
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);
    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_env_filter(filters)
            .finish()
            .with(fmt::Layer::default().with_writer(file_writer).with_ansi(false)),
    )?;

    let sp = span!(Level::TRACE, "start");
    let _g = sp.enter();

    tracing::debug!("test debug.");

    for i in 0..10 {
        test_write(i, "data------------data-----------------data");
    }

    Ok(())
}

#[instrument]
fn test_write(i: i32, data: &str) -> i32 {
    tracing::debug!("data:{data}");
    i
}
