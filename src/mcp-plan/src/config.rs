use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Runtime configuration for a single `mcp-plan` server.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct Config {
  #[serde(default)]
  pub database: DatabaseConfig,
  #[serde(default)]
  pub runtime: RuntimeConfig,
  #[serde(default)]
  pub sources: Vec<SourceConfig>,
}

/// Database configuration.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct DatabaseConfig {
  /// Database URL. Accepts `sqlite:data/mcp-plan.db`, `sqlite::memory:`,
  /// `postgres://user:password@host:port/database` or
  /// `mysql://user:password@host:port/database`.
  #[serde(default = "default_database_url")]
  pub url: String,
}

/// Estimated throughput and queueing knobs used by the MCP tools.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct RuntimeConfig {
  /// Estimated input throughput of the model, in tokens per second.
  #[serde(default = "default_tps_in")]
  pub tps_in: u64,
  /// Estimated output throughput of the model, in tokens per second.
  #[serde(default = "default_tps_out")]
  pub tps_out: u64,
  /// Estimated task duration above which a task is flagged for planning.
  #[serde(default = "default_max_task_duration_secs")]
  pub max_task_duration_secs: u64,
  /// Upper bound for the list returned by `queue()`.
  #[serde(default = "default_queue_limit")]
  pub queue_limit: usize,
  /// A task is escalated once its retry count reaches this value.
  #[serde(default = "default_max_retries")]
  pub max_retries: u32,
}

/// How a source is fed into the task graph.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars", rename_all = "lowercase")]
pub enum SourceType {
  #[default]
  Manual,
  Poll,
}

/// A statically configured source, synced into the `sources` table.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SourceConfig {
  pub id: String,
  pub title: String,
  #[serde(default)]
  pub description: String,
  #[serde(rename = "type", default)]
  #[schemars(rename = "type")]
  pub source_type: SourceType,
}

impl Default for DatabaseConfig {
  fn default() -> Self {
    Self {
      url: default_database_url(),
    }
  }
}

impl Default for RuntimeConfig {
  fn default() -> Self {
    Self {
      tps_in: default_tps_in(),
      tps_out: default_tps_out(),
      max_task_duration_secs: default_max_task_duration_secs(),
      queue_limit: default_queue_limit(),
      max_retries: default_max_retries(),
    }
  }
}

fn default_database_url() -> String {
  "sqlite:data/mcp-plan.db".to_owned()
}

fn default_tps_in() -> u64 {
  800
}

fn default_tps_out() -> u64 {
  28
}

fn default_max_task_duration_secs() -> u64 {
  600
}

fn default_queue_limit() -> usize {
  20
}

fn default_max_retries() -> u32 {
  3
}

#[derive(Parser, Debug)]
#[command(
  name = "mcp-plan",
  about = "MCP server that provides planning tooling."
)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Command,

  /// Path to the config file (defaults to `config.toml` in the current directory).
  #[arg(long, global = true)]
  pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
  Run,
  Migrate,
  /// Generate the JSON schema for the configuration.
  Schema {
    /// Output path
    #[arg(long)]
    output: PathBuf,
  },
}

impl Cli {
  pub fn resolve_config_path(&self) -> PathBuf {
    self
      .config
      .clone()
      .unwrap_or_else(|| PathBuf::from("config.toml"))
  }

  /// Load the configuration from `config.toml` (optional) overlaid with
  /// `MCP_PLAN_*` environment variables.
  pub fn load_config(&self) -> Result<Config> {
    let path = self.resolve_config_path();
    let file = ::config::File::from(path.as_path()).required(false);
    let env = ::config::Environment::with_prefix("MCP_PLAN").separator("__");
    let raw: ::config::Config = ::config::Config::builder()
      .add_source(file)
      .add_source(env)
      .build()
      .context("failed to build configuration")?;
    let config: Config = raw
      .try_deserialize()
      .context("failed to deserialize configuration")?;
    config.validate()?;
    Ok(config)
  }

  pub fn parse_command() -> (Self, Command) {
    let cli = Self::parse();
    let command = cli.command.clone();
    (cli, command)
  }
}

/// Generate the JSON schema for the configuration and write it to `path`.
pub fn generate_schema(path: &Path) -> Result<()> {
  let schema = rmcp::schemars::schema_for!(Config);
  let json = rmcp::serde_json::to_string_pretty(&schema)
    .context("failed to serialize schema")?;
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent).with_context(|| {
      format!("failed to create directory {}", parent.display())
    })?;
  }
  let contents = format!("{json}\n");
  std::fs::write(path, contents)
    .with_context(|| format!("failed to write schema to {}", path.display()))?;
  tracing::info!("wrote configuration schema to {}", path.display());
  Ok(())
}

impl Config {
  /// Reject configurations that would produce nonsensical throughput or
  /// duplicate source ids.
  fn validate(&self) -> Result<()> {
    if self.runtime.tps_in == 0 {
      bail!("runtime.tps_in must be greater than 0");
    }
    if self.runtime.tps_out == 0 {
      bail!("runtime.tps_out must be greater than 0");
    }
    if self.runtime.max_task_duration_secs == 0 {
      bail!("runtime.max_task_duration_secs must be greater than 0");
    }
    if self.runtime.queue_limit == 0 {
      bail!("runtime.queue_limit must be greater than 0");
    }

    let mut seen = HashSet::new();
    for source in &self.sources {
      if source.id.trim().is_empty() {
        bail!("source id must not be empty");
      }
      if !seen.insert(source.id.clone()) {
        bail!("duplicate source id: {}", source.id);
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::config::Environment;

  const FULL_TOML: &str = r#"
    [database]
    url = "sqlite:data/test.db"

    [runtime]
    tps_in = 100
    tps_out = 20
    max_task_duration_secs = 300
    queue_limit = 10
    max_retries = 2

    [[sources]]
    id = "gh"
    title = "GitHub issues"
    description = "fetch instructions"
    type = "poll"
  "#;

  fn from_toml(source: &str) -> Result<Config> {
    ::config::Config::builder()
      .add_source(::config::File::from_str(source, ::config::FileFormat::Toml))
      .build()
      .context("build")?
      .try_deserialize::<Config>()
      .context("deserialize")
  }

  #[test]
  fn defaults() -> Result<()> {
    let config = Config::default();
    assert_eq!(config.database.url, "sqlite:data/mcp-plan.db");
    assert_eq!(config.runtime.tps_in, default_tps_in());
    assert_eq!(config.runtime.tps_out, default_tps_out());
    assert_eq!(
      config.runtime.max_task_duration_secs,
      default_max_task_duration_secs()
    );
    assert_eq!(config.runtime.queue_limit, default_queue_limit());
    assert_eq!(config.runtime.max_retries, default_max_retries());
    assert!(config.sources.is_empty());
    Ok(())
  }

  #[test]
  fn parses_full_config() -> Result<()> {
    let config = from_toml(FULL_TOML)?;
    assert_eq!(config.database.url, "sqlite:data/test.db");
    assert_eq!(config.runtime.tps_in, 100);
    assert_eq!(config.runtime.max_retries, 2);
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].id, "gh");
    assert_eq!(config.sources[0].title, "GitHub issues");
    assert_eq!(config.sources[0].description, "fetch instructions");
    assert_eq!(config.sources[0].source_type, SourceType::Poll);
    Ok(())
  }

  #[test]
  fn missing_sections_use_defaults() -> Result<()> {
    let config = from_toml(
      r#"[runtime]
tps_in = 7"#,
    )?;
    assert_eq!(config.runtime.tps_in, 7);
    assert_eq!(config.runtime.tps_out, default_tps_out());
    assert_eq!(config.database.url, default_database_url());
    assert!(config.sources.is_empty());
    config.validate()?;
    Ok(())
  }

  #[test]
  fn empty_toml_uses_defaults() -> Result<()> {
    let config = from_toml("")?;
    assert_eq!(config.runtime.tps_in, default_tps_in());
    assert_eq!(config.database.url, default_database_url());
    Ok(())
  }

  #[test]
  fn rejects_zero_tps_in() -> Result<()> {
    let config = from_toml(
      r#"[runtime]
tps_in = 0"#,
    )?;
    assert!(config.validate().is_err());
    Ok(())
  }

  #[test]
  fn rejects_zero_queue_limit() -> Result<()> {
    let config = from_toml("[runtime]\nqueue_limit = 0")?;
    assert!(config.validate().is_err());
    Ok(())
  }

  #[test]
  fn rejects_duplicate_source_ids() -> Result<()> {
    let config = from_toml(
      r#"
      [[sources]]
      id = "a"
      title = "A"
      type = "manual"

      [[sources]]
      id = "a"
      title = "B"
      type = "manual"
      "#,
    )?;
    assert!(config.validate().is_err());
    Ok(())
  }

  #[test]
  fn generates_a_parseable_schema() -> Result<()> {
    let path = Path::new("target").join("schema-test.json");
    generate_schema(&path)?;
    let contents = std::fs::read_to_string(&path)?;
    let parsed: rmcp::serde_json::Value =
      rmcp::serde_json::from_str(&contents)?;
    assert!(parsed.is_object(), "schema should be a JSON object");
    assert_eq!(parsed["title"], "Config");
    let defs = parsed
      .get("$defs")
      .and_then(|v| v.as_object())
      .ok_or_else(|| anyhow::anyhow!("schema should carry type definitions"))?;
    for name in [
      "DatabaseConfig",
      "RuntimeConfig",
      "SourceConfig",
      "SourceType",
    ] {
      assert!(defs.contains_key(name), "schema should define `{name}`");
    }
    Ok(())
  }

  #[test]
  fn env_overrides_file() -> Result<()> {
    use std::collections::HashMap;

    let sources = Environment::default()
      .prefix("MCP_PLAN")
      .separator("__")
      .source(Some({
        let mut map = HashMap::new();
        map.insert("MCP_PLAN__RUNTIME__TPS_IN".to_owned(), "999".to_owned());
        map
      }));

    let config: Config = ::config::Config::builder()
      .add_source(::config::File::from_str(
        FULL_TOML,
        ::config::FileFormat::Toml,
      ))
      .add_source(sources)
      .build()
      .context("failed")?
      .try_deserialize()
      .context("deserialize")?;

    assert_eq!(config.runtime.tps_in, 999);
    assert_eq!(config.runtime.tps_out, 20);
    Ok(())
  }
}
