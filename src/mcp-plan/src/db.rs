pub mod entities {
  pub mod sources {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "sources")]
    pub struct Model {
      #[sea_orm(primary_key)]
      pub id: String,
      pub title: String,
      pub description: String,
      #[sea_orm(column_name = "type")]
      pub source_type: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
  }

  pub mod tasks {
    use sea_orm::entity::prelude::*;

    /// Database-native lifecycle status of a task. Mirrors
    /// `crate::models::TaskStatus`; convert via `From`.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "task_status")]
    pub enum TaskStatus {
      #[sea_orm(string_value = "ready")]
      Ready,
      #[sea_orm(string_value = "success")]
      Success,
      #[sea_orm(string_value = "failure")]
      Failure,
      #[sea_orm(string_value = "escalated")]
      Escalated,
    }

    /// Database-native priority of a task. Mirrors
    /// `crate::models::TaskPriority`; convert via `From`.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(
      rs_type = "String",
      db_type = "Enum",
      enum_name = "task_priority"
    )]
    pub enum TaskPriority {
      #[sea_orm(string_value = "critical")]
      Critical,
      #[sea_orm(string_value = "high")]
      High,
      #[sea_orm(string_value = "medium")]
      Medium,
      #[sea_orm(string_value = "low")]
      Low,
    }

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "tasks")]
    pub struct Model {
      #[sea_orm(primary_key)]
      pub id: String,
      pub parent_id: Option<String>,
      pub source_id: String,
      pub title: String,
      pub description: String,
      #[sea_orm(string_len = 255, nullable)]
      pub link: Option<String>,
      #[sea_orm(default_value = "ready")]
      pub status: TaskStatus,
      #[sea_orm(default_value = "medium")]
      pub priority: TaskPriority,
      #[sea_orm(default_value = 0)]
      pub retries: i32,
      pub created_at: ChronoDateTimeUtc,
      pub updated_at: ChronoDateTimeUtc,
      pub estimated_tokens_in: i64,
      pub estimated_tokens_reasoning: i64,
      pub estimated_tokens_out: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
      #[sea_orm(
        belongs_to = "crate::db::entities::sources::Entity",
        from = "Column::SourceId",
        to = "crate::db::entities::sources::Column::Id"
      )]
      Source,
      #[sea_orm(
        belongs_to = "crate::db::entities::tasks::Entity",
        from = "Column::ParentId",
        to = "Column::Id"
      )]
      Parent,
    }

    impl ActiveModelBehavior for ActiveModel {}
  }
}
