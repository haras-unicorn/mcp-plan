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
    // sqlx's sqlite driver does not create a missing database file unless
    // `create_if_missing` is set. sea-orm does not expose it, so create the
    // file (and its parent directory) here, before connecting.
    prepare_sqlite(url)?;
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

/// Extract the file path out of a `sqlite:` url, supporting both the bare
/// (`sqlite:data/plan.db`, `sqlite:/var/lib/plan.db`) and the proper URI
/// (`sqlite://data/plan.db`, `sqlite:///var/lib/plan.db`) forms.
#[cfg(feature = "sqlite")]
fn sqlite_path(url: &str) -> PathBuf {
  let rest = url.strip_prefix("sqlite:").unwrap_or(url);
  let rest = rest.strip_prefix(':').unwrap_or(rest);

  if let Some(rest) = rest.strip_prefix("//") {
    // Proper URI form: `sqlite://[authority]/path`. An empty authority with a
    // leading `/` (i.e. `sqlite:///abs/path`) denotes an absolute path; a
    // non-empty authority (`sqlite://data/plan.db`) is a leading path segment.
    if let Some(path) = rest.strip_prefix('/') {
      PathBuf::from(format!("/{path}"))
    } else {
      PathBuf::from(rest)
    }
  } else {
    PathBuf::from(rest)
  }
}

/// Create the parent directory and the database file (if missing) for a
/// file-backed sqlite url.
#[cfg(feature = "sqlite")]
fn prepare_sqlite(url: &str) -> anyhow::Result<()> {
  let path = sqlite_path(url);

  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent).with_context(|| {
      format!("failed to create database directory {}", parent.display())
    })?;
  }

  if !path.exists() {
    std::fs::OpenOptions::new()
      .create(true)
      .truncate(false)
      .write(true)
      .open(&path)
      .with_context(|| {
        format!("failed to create database file {}", path.display())
      })?;
  }

  Ok(())
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
  use super::*;
  use std::path::Path;

  struct TestDir(PathBuf);

  impl TestDir {
    fn new(sub: &str) -> Self {
      let root = Path::new("target").join(format!("connect-test-{sub}"));
      let _ = std::fs::remove_dir_all(&root);
      let _ = std::fs::create_dir_all(&root);
      Self(root)
    }

    fn join(&self, p: &str) -> PathBuf {
      self.0.join(p)
    }
  }

  impl Drop for TestDir {
    fn drop(&mut self) {
      let _ = std::fs::remove_dir_all(&self.0);
    }
  }

  #[test]
  fn parses_bare_and_uri_forms() {
    assert_eq!(
      sqlite_path("sqlite:data/plan.db"),
      PathBuf::from("data/plan.db")
    );
    assert_eq!(
      sqlite_path("sqlite://data/plan.db"),
      PathBuf::from("data/plan.db")
    );
    assert_eq!(
      sqlite_path("sqlite:///var/lib/plan.db"),
      PathBuf::from("/var/lib/plan.db")
    );
    assert_eq!(
      sqlite_path("sqlite:/var/lib/plan.db"),
      PathBuf::from("/var/lib/plan.db")
    );
  }

  #[tokio::test]
  async fn opens_a_file_that_does_not_exist() -> anyhow::Result<()> {
    let dir = TestDir::new("missing");
    let relative = dir.join("nested").join("plan.db");
    let url = format!("sqlite://{}", relative.display());

    let config = Config {
      database: crate::config::DatabaseConfig { url: url.clone() },
      ..Config::default()
    };

    let db = connect(&config).await?;
    db.close().await?;

    assert!(
      relative.exists(),
      "connect should create the database file at {relative:?}"
    );
    Ok(())
  }

  #[tokio::test]
  async fn reconnects_to_existing_file() -> anyhow::Result<()> {
    let dir = TestDir::new("existing");
    let relative = dir.join("plan.db");
    let url = format!("sqlite://{}", relative.display());

    let config = Config {
      database: crate::config::DatabaseConfig { url: url.clone() },
      ..Config::default()
    };

    let db = connect(&config).await?;
    db.close().await?;
    let db = connect(&config).await?;
    db.close().await?;
    Ok(())
  }
}
