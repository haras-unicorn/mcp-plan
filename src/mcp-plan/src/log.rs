use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber for the application.
///
/// Logs are emitted as flattened newline-delimited JSON on **stderr** so they
/// never interfere with the MCP JSON-RPC protocol, which owns stdout. Event
/// fields are hoisted to the root of each record, making every line readable
/// directly with `jq` (e.g. `jq -r '.message'`). Filtering is driven entirely by
/// `RUST_LOG` (defaulting to `info`); configuration stays code-level.
pub fn init() {
  let env_filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info"));
  tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .with_env_filter(env_filter)
    .json()
    .flatten_event(true)
    .with_current_span(false)
    .init();
}

/// A short human-readable summary of the database backend, derived from the
/// URL. Never includes the URL itself, which may contain credentials.
pub fn backend_label(url: &str) -> &'static str {
  if url.starts_with("sqlite://") {
    "sqlite"
  } else if url.starts_with("postgres://") {
    "postgres"
  } else if url.starts_with("mysql://") {
    "mysql"
  } else {
    "unknown"
  }
}
