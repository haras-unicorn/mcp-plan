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
  let config = cli.load_config()?;

  match command {
    config::Command::Run => {
      let db = connect::connect(&config).await?;
      let service = Service::new(db, Arc::new(config));
      let server = PlanServer { service };
      let service = server.serve(stdio()).await?;
      service.waiting().await?;
    }
    config::Command::Migrate => {
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
