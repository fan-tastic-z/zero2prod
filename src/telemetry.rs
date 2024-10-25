use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

pub fn init_tracing() {
    // console layer for tracing-subscriber
    let console = fmt::Layer::new()
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .with_filter(LevelFilter::DEBUG);

    // file appender layer for tracing-subscriber
    let file_appender = tracing_appender::rolling::daily("./", "zero2prod.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file = fmt::Layer::new()
        .with_writer(non_blocking)
        .json()
        .with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(console)
        .with(file)
        .init();
}
