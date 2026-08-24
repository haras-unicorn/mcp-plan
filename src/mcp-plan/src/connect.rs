use crate::config::Config;
use crate::migration::Migrator;
use anyhow::Context;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;

/// Open the database from `config.database.url`, enable sqlite pragmas when the
/// url targets sqlite, then apply any pending migrations.
pub async fn connect(config: &Config) -> anyhow::Result<DatabaseConnection> {
  let url = &config.database.url;
  let backend = crate::log::backend_label(url);
  tracing::debug!(backend, "opening database");

  #[cfg(feature = "sqlite")]
  if is_sqlite(url) && !is_sqlite_memory(url) {
    prepare_sqlite_dir(url)?;
  }

  let db = sea_orm::Database::connect(url)
    .await
    .with_context(|| format!("failed to connect to `{backend}` database"))?;

  #[cfg(feature = "sqlite")]
  if is_sqlite(url) {
    db.execute_unprepared("PRAGMA journal_mode = WAL;").await?;
    db.execute_unprepared("PRAGMA foreign_keys = ON;").await?;
  }

  Migrator::up(&db, None).await?;
  tracing::debug!(backend, "database ready");
  Ok(db)
}

/// Whether `url` points at a sqlite database.
#[cfg(feature = "sqlite")]
fn is_sqlite(url: &str) -> bool {
  url.starts_with("sqlite:")
}

/// Whether the sqlite url is an in-memory database (nothing to prepare on disk).
#[cfg(feature = "sqlite")]
fn is_sqlite_memory(url: &str) -> bool {
  url == "sqlite::memory:" || url.contains(":memory:")
}

/// Extract the file path out of a `sqlite:` url.
#[cfg(feature = "sqlite")]
fn sqlite_path(url: &str) -> PathBuf {
  let rest = url.strip_prefix("sqlite:").unwrap_or(url);
  let rest = rest.strip_prefix(':').unwrap_or(rest);
  PathBuf::from(rest)
}

/// Create the parent directory of a sqlite file if it doesn't exist yet.
#[cfg(feature = "sqlite")]
fn prepare_sqlite_dir(url: &str) -> anyhow::Result<()> {
  if let Some(parent) = sqlite_path(url).parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent).with_context(|| {
      format!("failed to create database directory {}", parent.display())
    })?;
  }
  Ok(())
}
