use tracing_subscriber::EnvFilter;

/// Initialize tracing subscriber for the application.
pub fn init() {
  let env_filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info"));
  tracing_subscriber::fmt().with_env_filter(env_filter).init();
}
