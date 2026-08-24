pub mod entities {
  pub mod sources {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "sources")]
    pub struct Model {
      #[sea_orm(primary_key)]
      pub id: String,
      pub title: String,
      #[sea_orm(default)]
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

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "tasks")]
    pub struct Model {
      #[sea_orm(primary_key)]
      pub id: String,
      pub parent_id: Option<String>,
      pub source_id: Option<String>,
      pub title: String,
      pub description: String,
      #[sea_orm(default_value = "ready")]
      pub status: String,
      #[sea_orm(default_value = "medium")]
      pub priority: String,
      #[sea_orm(default_value = 0)]
      pub retries: i32,
      pub created_at: Option<ChronoDateTimeUtc>,
      pub updated_at: Option<ChronoDateTimeUtc>,
      pub estimated_tokens_in: Option<i64>,
      pub estimated_tokens_reasoning: Option<i64>,
      pub estimated_tokens_out: Option<i64>,
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
