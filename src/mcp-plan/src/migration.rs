use sea_orm::{DbBackend, Schema};
use sea_orm_migration::prelude::*;

fn schema_for<E: sea_orm::EntityTrait>(
  entity: E,
  backend: DbBackend,
) -> TableCreateStatement {
  Schema::new(backend).create_table_from_entity(entity)
}

/// Runs pending migrations against a database.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(CreateInitialSchema)]
  }
}

/// Creates the `sources` and `tasks` tables.
#[derive(DeriveMigrationName)]
pub struct CreateInitialSchema;

#[async_trait::async_trait]
impl MigrationTrait for CreateInitialSchema {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    manager
      .create_table(schema_for(crate::db::entities::sources::Entity, backend))
      .await?;
    manager
      .create_table(schema_for(crate::db::entities::tasks::Entity, backend))
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table("tasks").to_owned())
      .await?;
    manager
      .drop_table(Table::drop().table("sources").to_owned())
      .await
  }
}

/// Convenience wrapper around [`Migrator`].
pub struct MigrationRunner(pub sea_orm::DatabaseConnection);

impl MigrationRunner {
  pub fn new(db: sea_orm::DatabaseConnection) -> Self {
    Self(db)
  }

  pub async fn up(&self) -> Result<(), DbErr> {
    Migrator::up(&self.0, None).await
  }

  pub async fn down(&self) -> Result<(), DbErr> {
    Migrator::down(&self.0, None).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sea_orm::entity::prelude::*;
  use sea_orm::{ActiveValue, DatabaseConnection};

  /// Run the full up → insert-with-defaults → down cycle against a connection.
  async fn assert_migration_cycle(
    db: &DatabaseConnection,
  ) -> Result<(), Box<dyn std::error::Error>> {
    Migrator::up(db, None).await?;

    let manager = SchemaManager::new(db);
    for table in ["sources", "tasks"] {
      assert!(
        manager.has_table(table).await?,
        "table `{table}` should exist after migration"
      );
    }

    let task = crate::db::entities::tasks::ActiveModel {
      id: ActiveValue::Set("t1".to_owned()),
      parent_id: ActiveValue::NotSet,
      source_id: ActiveValue::NotSet,
      title: ActiveValue::Set("title".to_owned()),
      description: ActiveValue::Set("desc".to_owned()),
      status: ActiveValue::NotSet,
      priority: ActiveValue::NotSet,
      retries: ActiveValue::NotSet,
      created_at: ActiveValue::NotSet,
      updated_at: ActiveValue::NotSet,
      estimated_tokens_in: ActiveValue::NotSet,
      estimated_tokens_reasoning: ActiveValue::NotSet,
      estimated_tokens_out: ActiveValue::NotSet,
    };
    let saved = task.insert(db).await?;
    assert_eq!(saved.status, "ready", "status should default to `ready`");
    assert_eq!(
      saved.priority, "medium",
      "priority should default to `medium`"
    );
    assert_eq!(saved.retries, 0, "retries should default to 0");

    Migrator::down(db, None).await?;

    let manager = SchemaManager::new(db);
    for table in ["sources", "tasks"] {
      assert!(
        !manager.has_table(table).await?,
        "table `{table}` should be dropped by the down migration"
      );
    }

    Ok(())
  }

  #[tokio::test]
  async fn migrate_up_and_down_sqlite() -> Result<(), Box<dyn std::error::Error>>
  {
    let db = sea_orm::Database::connect("sqlite::memory:").await?;
    assert_migration_cycle(&db).await
  }

  #[cfg(feature = "postgres")]
  #[tokio::test]
  async fn migrate_up_and_down_postgres()
  -> Result<(), Box<dyn std::error::Error>> {
    use testcontainers_modules::postgres::Postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = Postgres::default().start().await?;
    let url = format!(
      "postgres://postgres:postgres@{}:{}/postgres",
      container.get_host().await?,
      container.get_host_port_ipv4(5432).await?
    );
    let db = sea_orm::Database::connect(&url).await?;
    assert_migration_cycle(&db).await
  }

  #[cfg(feature = "mysql")]
  #[tokio::test]
  async fn migrate_up_and_down_mysql() -> Result<(), Box<dyn std::error::Error>>
  {
    use testcontainers_modules::mysql::Mysql;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = Mysql::default().start().await?;
    let url = format!(
      "mysql://root@{}:{}/test",
      container.get_host().await?,
      container.get_host_port_ipv4(3306).await?
    );
    let db = sea_orm::Database::connect(&url).await?;
    assert_migration_cycle(&db).await
  }
}
