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
    // Postgres stores enum columns as native types which must exist before
    // the tables referencing them are created. SQLite and MySQL inline the
    // enum into the column definition and generate no statements here.
    if backend == DbBackend::Postgres {
      for statement in Schema::new(backend)
        .create_enum_from_entity(crate::db::entities::tasks::Entity)
      {
        manager.exec_stmt(statement).await?;
      }
    }
    manager
      .create_table(schema_for(crate::db::entities::sources::Entity, backend))
      .await?;
    manager
      .create_table(schema_for(crate::db::entities::tasks::Entity, backend))
      .await?;
    manager
      .create_index(
        Index::create()
          .name("idx-tasks-link")
          .table(crate::db::entities::tasks::Entity)
          .col(crate::db::entities::tasks::Column::Link)
          .unique()
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table("tasks").to_owned())
      .await?;
    manager
      .drop_table(Table::drop().table("sources").to_owned())
      .await?;

    // Drop the postgres enum types created in `up`, now that no table
    // references them any more. SQLite and MySQL have no separate types.
    let backend = manager.get_database_backend();
    if backend == DbBackend::Postgres {
      for name in ["task_status", "task_priority"] {
        manager
          .drop_type(
            extension::postgres::Type::drop()
              .name(name)
              .if_exists()
              .to_owned(),
          )
          .await?;
      }
    }

    Ok(())
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

    let source = crate::db::entities::sources::ActiveModel {
      id: ActiveValue::Set("s1".to_owned()),
      title: ActiveValue::Set("title".to_owned()),
      description: ActiveValue::Set("desc".to_owned()),
      source_type: ActiveValue::Set("manual".to_owned()),
    };
    let saved_source = source.insert(db).await?;

    let task = crate::db::entities::tasks::ActiveModel {
      id: ActiveValue::Set("t1".to_owned()),
      parent_id: ActiveValue::NotSet,
      source_id: ActiveValue::Set(saved_source.id),
      title: ActiveValue::Set("title".to_owned()),
      description: ActiveValue::Set("desc".to_owned()),
      link: ActiveValue::NotSet,
      status: ActiveValue::NotSet,
      priority: ActiveValue::NotSet,
      retries: ActiveValue::NotSet,
      created_at: ActiveValue::Set(ChronoUtc::now()),
      updated_at: ActiveValue::Set(ChronoUtc::now()),
      estimated_tokens_in: ActiveValue::Set(0),
      estimated_tokens_reasoning: ActiveValue::Set(0),
      estimated_tokens_out: ActiveValue::Set(0),
    };
    let saved_task = task.insert(db).await?;
    assert_eq!(
      saved_task.status,
      crate::db::entities::tasks::TaskStatus::Ready,
      "status should default to `ready`"
    );
    assert_eq!(
      saved_task.priority,
      crate::db::entities::tasks::TaskPriority::Medium,
      "priority should default to `medium`"
    );
    assert_eq!(saved_task.retries, 0, "retries should default to 0");

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

  #[cfg(feature = "sqlite")]
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
    if !crate::test::docker_available().await {
      eprintln!("docker is not available, skipping postgres test");
      return Ok(());
    }

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
    if !crate::test::docker_available().await {
      eprintln!("docker is not available, skipping mysql test");
      return Ok(());
    }

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
