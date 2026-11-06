use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing() -> Option<WorkerGuard> {
    if cfg!(debug_assertions) {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug,tower_http=debug,axum=trace"));

        let terminal_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_target(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(terminal_layer)
            .init();

        None
    } else {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));

        // Only save logs from the last week
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("app.log")
            .max_log_files(7)
            .build("logs")
            .expect("failed to create log file appender");

        let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking_file)
            .with_ansi(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();

        Some(guard)
    }
}
