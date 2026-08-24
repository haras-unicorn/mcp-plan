#![deny(unsafe_code)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable
)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::todo)]
#![deny(clippy::allow_attributes_without_reason)]

use mcp_plan::{PlanServer, config, connect, log, service::Service};
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  log::init();

  let (cli, command) = config::Cli::parse_command();
  tracing::debug!(?command, config = %cli.resolve_config_path().display(), "parsed command and configuration");
  let config = cli.load_config()?;

  match &command {
    config::Command::Run => {}
    config::Command::Migrate => {}
    config::Command::Schema { .. } => {}
  }
  match command {
    config::Command::Run => {
      tracing::info!("starting mcp-plan server");
      let db = connect::connect(&config).await?;
      let service = Service::new(db, Arc::new(config));
      let server = PlanServer { service };
      let service = server.serve(stdio()).await?;
      tracing::info!("mcp-plan server served over stdio");
      service.waiting().await?;
      tracing::info!("shutting down mcp-plan server");
    }
    config::Command::Migrate => {
      tracing::info!("running database migrations");
      let db = connect::connect(&config).await?;
      drop(db);
      tracing::info!("Migrations completed successfully.");
    }
    config::Command::Schema { output } => {
      config::generate_schema(&output)?;
    }
  }

  Ok(())
}
