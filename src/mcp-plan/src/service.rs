//! Thin DB access layer. The [`Service`] owns the connection and a shared,
//! read-only [`Config`], returning only plain DTOs from `crate::models` so
//! sea-orm types stay out of the tools.

use crate::config::{Config, RuntimeConfig};
use crate::db::entities::tasks;
use crate::models::{
  NewTask, QueuedTask, Task, TaskPriority, TaskStatus, TaskSummary, TaskType,
  TaskUpdate,
};
use chrono::Utc;
use sea_orm::ActiveModelTrait as _;
use sea_orm::{
  ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
  QueryFilter, QueryOrder,
};
use std::sync::Arc;

/// Error returned by the service layer. Converted to a structured tool error
/// by the tool handlers.
#[derive(Debug)]
pub enum ServiceError {
  NotFound(String),
  Db(sea_orm::DbErr),
}

impl std::fmt::Display for ServiceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NotFound(msg) => write!(f, "{msg}"),
      Self::Db(err) => write!(f, "database error: {err}"),
    }
  }
}

impl std::error::Error for ServiceError {}

impl From<sea_orm::DbErr> for ServiceError {
  fn from(value: sea_orm::DbErr) -> Self {
    Self::Db(value)
  }
}

/// Service layer over the task database. Holds the connection and the shared
/// runtime configuration so callers don't pass state on every call.
#[derive(Clone, Debug)]
pub struct Service {
  pub db: DatabaseConnection,
  pub config: Arc<Config>,
}

impl Service {
  pub fn new(db: DatabaseConnection, config: Arc<Config>) -> Self {
    Self { db, config }
  }

  /// Fetch a single task by id.
  pub async fn task(&self, id: &str) -> Result<Option<Task>, ServiceError> {
    let model = tasks::Entity::find_by_id(id.to_owned())
      .one(&self.db)
      .await?;
    Ok(model.map(Task::from))
  }

  /// Fetch direct children of `parent_id`. `None` means root-level tasks.
  pub async fn children(
    &self,
    parent_id: Option<&str>,
  ) -> Result<Vec<TaskSummary>, ServiceError> {
    let mut query = tasks::Entity::find();
    match parent_id {
      Some(parent) => {
        query = query.filter(tasks::Column::ParentId.eq(parent));
      }
      None => {
        query = query.filter(tasks::Column::ParentId.is_null());
      }
    }
    let models = query
      .order_by_asc(tasks::Column::CreatedAt)
      .all(&self.db)
      .await?;
    Ok(models.iter().map(TaskSummary::from).collect())
  }

  /// Insert a new task. Returns the generated id. Status starts at `ready`.
  pub async fn insert(
    &self,
    new_task: NewTask,
    parent_id: Option<&str>,
  ) -> Result<String, ServiceError> {
    if let Some(parent) = parent_id
      && tasks::Entity::find_by_id(parent.to_owned())
        .one(&self.db)
        .await?
        .is_none()
    {
      return Err(ServiceError::NotFound(format!(
        "parent `{parent}` not found"
      )));
    }

    let now = Utc::now();
    let model = tasks::ActiveModel {
      id: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
      parent_id: ActiveValue::Set(parent_id.map(ToOwned::to_owned)),
      source_id: ActiveValue::Set(new_task.source_id),
      title: ActiveValue::Set(new_task.title),
      description: ActiveValue::Set(new_task.description),
      status: ActiveValue::Set(TaskStatus::Ready.as_str().to_owned()),
      priority: ActiveValue::Set(new_task.priority.as_str().to_owned()),
      retries: ActiveValue::Set(0),
      created_at: ActiveValue::Set(Some(now)),
      updated_at: ActiveValue::Set(Some(now)),
      estimated_tokens_in: ActiveValue::Set(new_task.estimated_tokens_in),
      estimated_tokens_reasoning: ActiveValue::Set(
        new_task.estimated_tokens_reasoning,
      ),
      estimated_tokens_out: ActiveValue::Set(new_task.estimated_tokens_out),
    };
    let saved = model.insert(&self.db).await?;
    Ok(saved.id)
  }

  /// Mark a task as `success` or `failure` (per `complete()`).
  pub async fn complete(
    &self,
    task_id: &str,
    status: TaskStatus,
  ) -> Result<Task, ServiceError> {
    if status == TaskStatus::Ready || status == TaskStatus::Escalated {
      return Err(ServiceError::NotFound(
        "`complete` only accepts `success` or `failure`".to_owned(),
      ));
    }
    self.update_status(task_id, status).await
  }

  /// Increment `retries` and mark as `failure` (per `fail()`). When retries
  /// reach the configured `RuntimeConfig::max_retries`, the task is escalated
  /// instead. Returns the updated task.
  pub async fn fail(&self, task_id: &str) -> Result<Task, ServiceError> {
    let model = self.load(task_id).await?;
    let retries = model.retries.saturating_add(1);
    let limit =
      i32::try_from(self.config.runtime.max_retries).unwrap_or(i32::MAX);
    let status = if retries >= limit {
      TaskStatus::Escalated
    } else {
      TaskStatus::Failure
    };

    let mut am: tasks::ActiveModel = model.into();
    am.retries = ActiveValue::Set(retries);
    am.status = ActiveValue::Set(status.as_str().to_owned());
    am.updated_at = ActiveValue::Set(Some(Utc::now()));
    let saved = am.update(&self.db).await?;
    Ok(Task::from(saved))
  }

  /// Mark a task as `escalated`.
  pub async fn escalate(&self, task_id: &str) -> Result<Task, ServiceError> {
    self.update_status(task_id, TaskStatus::Escalated).await
  }

  /// Update mutable fields and set status back to `ready`.
  pub async fn ready(
    &self,
    task_id: &str,
    update: TaskUpdate,
  ) -> Result<Task, ServiceError> {
    let model = tasks::Entity::find_by_id(task_id.to_owned())
      .one(&self.db)
      .await?
      .ok_or_else(|| {
        ServiceError::NotFound(format!("task `{task_id}` not found"))
      })?;
    let mut am: tasks::ActiveModel = model.into();
    if let Some(title) = update.title {
      am.title = ActiveValue::Set(title);
    }
    if let Some(description) = update.description {
      am.description = ActiveValue::Set(description);
    }
    if let Some(priority) = update.priority {
      am.priority = ActiveValue::Set(priority.as_str().to_owned());
    }
    if let Some(v) = update.estimated_tokens_in {
      am.estimated_tokens_in = ActiveValue::Set(Some(v));
    }
    if let Some(v) = update.estimated_tokens_reasoning {
      am.estimated_tokens_reasoning = ActiveValue::Set(Some(v));
    }
    if let Some(v) = update.estimated_tokens_out {
      am.estimated_tokens_out = ActiveValue::Set(Some(v));
    }
    am.status = ActiveValue::Set(TaskStatus::Ready.as_str().to_owned());
    am.updated_at = ActiveValue::Set(Some(Utc::now()));
    let saved = am.update(&self.db).await?;
    Ok(Task::from(saved))
  }

  /// The `queue()` engine: returns tasks needing work, sorted by priority then
  /// type (planning → ready), bounded by `config.queue_limit`.
  pub async fn queue(&self) -> Result<Vec<QueuedTask>, ServiceError> {
    let config = &self.config.runtime;
    let max_retries =
      i32::try_from(config.max_retries).unwrap_or(i32::MAX);
    let models = tasks::Entity::find()
      .filter(
        Condition::all()
          .add(tasks::Column::Status.ne(TaskStatus::Success.as_str()))
          .add(tasks::Column::Status.ne(TaskStatus::Escalated.as_str())),
      )
      .all(&self.db)
      .await?;

    let mut result: Vec<QueuedTask> = Vec::with_capacity(models.len());
    for model in models {
      if model.status == TaskStatus::Failure.as_str()
        && model.retries >= max_retries
      {
        let mut am: tasks::ActiveModel = model.into();
        am.status = ActiveValue::Set(TaskStatus::Escalated.as_str().to_owned());
        am.updated_at = ActiveValue::Set(Some(Utc::now()));
        am.update(&self.db).await?;
        continue;
      }

      let duration_secs = calculate_duration(&model, config);
      let kind = if duration_secs > config.max_task_duration_secs {
        TaskType::Planning
      } else {
        TaskType::Execution
      };
      result.push(QueuedTask {
        duration_secs,
        kind,
        task: Task::from(model),
      });
    }

    result.sort_by(|a, b| {
      priority_rank(a.task.priority)
        .cmp(&priority_rank(b.task.priority))
        .then_with(|| kind_rank(a.kind).cmp(&kind_rank(b.kind)))
        .then_with(|| a.task.id.cmp(&b.task.id))
    });

    result.truncate(config.queue_limit);
    Ok(result)
  }

  async fn load(&self, task_id: &str) -> Result<tasks::Model, ServiceError> {
    tasks::Entity::find_by_id(task_id.to_owned())
      .one(&self.db)
      .await?
      .ok_or_else(|| {
        ServiceError::NotFound(format!("task `{task_id}` not found"))
      })
  }

  async fn update_status(
    &self,
    task_id: &str,
    status: TaskStatus,
  ) -> Result<Task, ServiceError> {
    let model = self.load(task_id).await?;
    let mut am: tasks::ActiveModel = model.into();
    am.status = ActiveValue::Set(status.as_str().to_owned());
    am.updated_at = ActiveValue::Set(Some(Utc::now()));
    let saved = am.update(&self.db).await?;
    Ok(Task::from(saved))
  }
}

/// Compute the expected wall-clock duration of a task from its token
/// estimates and the configured throughput (see design.md).
pub fn calculate_duration(
  model: &tasks::Model,
  config: &RuntimeConfig,
) -> u64 {
  let tps_in = i64::try_from(config.tps_in).unwrap_or(i64::MAX).max(1);
  let tps_out = i64::try_from(config.tps_out).unwrap_or(i64::MAX).max(1);

  let in_tokens = model.estimated_tokens_in.unwrap_or(0);
  let out_tokens = model
    .estimated_tokens_reasoning
    .unwrap_or(0)
    .saturating_add(model.estimated_tokens_out.unwrap_or(0));

  let in_secs = in_tokens.checked_div(tps_in).unwrap_or(0);
  let out_secs = out_tokens.checked_div(tps_out).unwrap_or(0);
  u64::try_from(in_secs.saturating_add(out_secs)).unwrap_or(u64::MAX)
}

fn priority_rank(priority: TaskPriority) -> u8 {
  match priority {
    TaskPriority::Critical => 0,
    TaskPriority::High => 1,
    TaskPriority::Medium => 2,
    TaskPriority::Low => 3,
  }
}

fn kind_rank(kind: TaskType) -> u8 {
  match kind {
    TaskType::Planning => 0,
    TaskType::Execution => 1,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::DatabaseConfig;
  use crate::migration::Migrator;
  use sea_orm_migration::MigratorTrait;

  fn config() -> RuntimeConfig {
    RuntimeConfig {
      tps_in: 100,
      tps_out: 20,
      max_task_duration_secs: 40,
      queue_limit: 10,
      max_retries: 2,
    }
  }

  async fn new() -> Service {
    let db = sea_orm::Database::connect("sqlite::memory:")
      .await
      .expect("db");
    Migrator::up(&db, None).await.expect("migrate");
    let config = Config {
      database: DatabaseConfig::default(),
      runtime: config(),
      sources: Vec::new(),
    };
    Service::new(db, Arc::new(config))
  }

  async fn seed_root(service: &Service, title: &str) -> String {
    seed_with_tokens(service, title, None, None).await
  }

  async fn seed_with_tokens(
    service: &Service,
    title: &str,
    stdin: Option<i64>,
    stdout: Option<i64>,
  ) -> String {
    service
      .insert(
        NewTask {
          title: title.to_owned(),
          description: "".to_owned(),
          priority: TaskPriority::Medium,
          source_id: None,
          estimated_tokens_in: stdin,
          estimated_tokens_reasoning: None,
          estimated_tokens_out: stdout,
        },
        None,
      )
      .await
      .expect("insert")
  }

  #[tokio::test]
  async fn insert_and_fetch() {
    let service = new().await;
    let id =
      seed_with_tokens(&service, "root", Some(100), None).await;
    let fetched = service.task(&id).await.expect("fetch").expect("some");
    assert_eq!(fetched.title, "root");
    assert_eq!(fetched.status, TaskStatus::Ready);
    assert_eq!(fetched.priority, TaskPriority::Medium);
    assert_eq!(fetched.retries, 0);
    assert!(fetched.created_at.is_some());
    assert!(fetched.updated_at.is_some());
  }

  #[tokio::test]
  async fn root_children_and_scoped_children() {
    let service = new().await;
    let root = seed_root(&service, "root").await;
    let child_a = service
      .insert(
        NewTask {
          title: "a".to_owned(),
          description: "".to_owned(),
          priority: TaskPriority::High,
          source_id: None,
          estimated_tokens_in: None,
          estimated_tokens_reasoning: None,
          estimated_tokens_out: None,
        },
        Some(&root),
      )
      .await
      .expect("child a");
    let child_b = service
      .insert(
        NewTask {
          title: "b".to_owned(),
          description: "".to_owned(),
          priority: TaskPriority::Low,
          source_id: None,
          estimated_tokens_in: None,
          estimated_tokens_reasoning: None,
          estimated_tokens_out: None,
        },
        Some(&root),
      )
      .await
      .expect("child b");

    let roots: Vec<TaskSummary> =
      service.children(None).await.expect("roots");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, root);

    let kids: Vec<TaskSummary> =
      service.children(Some(&root)).await.expect("kids");
    assert_eq!(kids.len(), 2);
    let by_title: std::collections::HashMap<String, &TaskSummary> =
      kids.iter().map(|k| (k.title.clone(), k)).collect();
    assert_eq!(by_title["a"].id, child_a);
    assert_eq!(by_title["b"].id, child_b);
  }

  #[tokio::test]
  async fn complete_sets_status() {
    let service = new().await;
    let id = seed_root(&service, "root").await;
    let _ = service
      .complete(&id, TaskStatus::Success)
      .await
      .expect("complete");
    let fetched = service.task(&id).await.expect("fetch").expect("some");
    assert_eq!(fetched.status, TaskStatus::Success);
  }

  #[tokio::test]
  async fn fail_increments_and_escalates_at_limit() {
    let service = new().await;
    let id = seed_root(&service, "root").await;

    let f1 = service.fail(&id).await.expect("fail 1");
    assert_eq!(f1.retries, 1);
    assert_eq!(f1.status, TaskStatus::Failure);

    let f2 = service.fail(&id).await.expect("fail 2");
    assert_eq!(f2.retries, 2);
    assert_eq!(f2.status, TaskStatus::Escalated);
  }

  #[tokio::test]
  async fn ready_updates_fields_and_status() {
    let service = new().await;
    let id = seed_root(&service, "root").await;
    let updated = service
      .ready(
        &id,
        TaskUpdate {
          title: Some("renamed".to_owned()),
          priority: Some(TaskPriority::Critical),
          estimated_tokens_in: Some(5000),
          estimated_tokens_out: Some(2000),
          ..TaskUpdate::default()
        },
      )
      .await
      .expect("ready");
    assert_eq!(updated.title, "renamed");
    assert_eq!(updated.priority, TaskPriority::Critical);
    assert_eq!(updated.status, TaskStatus::Ready);
    assert_eq!(updated.estimated_tokens_in, Some(5000));
  }

  #[tokio::test]
  async fn queue_sorts_and_classifies() {
    let service = new().await;
    // Long task -> planning
    let p_id = seed_with_tokens(
      &service,
      "planning",
      Some(60_000),
      Some(10_000),
    )
    .await;
    // Short low-priority
    let e_id = seed_with_tokens(&service, "execution", Some(10), None).await;

    let queued = service.queue().await.expect("queue");
    assert_eq!(queued.len(), 2);
    let kinds: Vec<TaskType> = queued.iter().map(|q| q.kind).collect();
    assert!(kinds.contains(&TaskType::Planning));
    assert!(kinds.contains(&TaskType::Execution));
    // Planning (long) should sort before execution; same priority, planning first.
    let ids: Vec<&str> = queued.iter().map(|q| q.task.id.as_str()).collect();
    assert_eq!(ids[0], p_id);
    assert_eq!(ids[1], e_id);
  }

  #[tokio::test]
  async fn queue_excludes_success_and_escalated() {
    let service = new().await;
    let done = seed_root(&service, "done").await;
    let esc = seed_root(&service, "esc").await;
    let _ = service
      .complete(&done, TaskStatus::Success)
      .await
      .expect("done");
    let _ = service.escalate(&esc).await.expect("esc");
    let _ = seed_root(&service, "active").await;

    let queued = service.queue().await.expect("queue");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].task.title, "active");
  }

  #[tokio::test]
  async fn queue_truncates_to_limit() {
    let service = new().await;
    for i in 0..25 {
      seed_root(&service, &format!("t{i}")).await;
    }
    let queued = service.queue().await.expect("queue");
    assert_eq!(queued.len(), 10);
  }

  #[tokio::test]
  async fn queue_escalates_exhausted_failures() {
    let service = new().await;
    let id = seed_root(&service, "sick").await;

    use sea_orm::Set;
    let model = service.load(&id).await.expect("load");
    let mut am: tasks::ActiveModel = model.into();
    am.retries = Set(2);
    am.status = Set("failure".to_owned());
    am.update(&service.db).await.expect("update");

    let queued = service.queue().await.expect("queue");
    assert!(
      queued.is_empty(),
      "exhausted failure should be escalated out of the queue"
    );

    let fetched = service.load(&id).await.expect("fetch");
    assert_eq!(fetched.status, "escalated");
  }

  #[test]
  fn duration_formula() {
    let cfg = config();
    let model = tasks::Model {
      id: "t".to_owned(),
      parent_id: None,
      source_id: None,
      title: "".to_owned(),
      description: "".to_owned(),
      status: "ready".to_owned(),
      priority: "medium".to_owned(),
      retries: 0,
      created_at: None,
      updated_at: None,
      estimated_tokens_in: Some(100),
      estimated_tokens_reasoning: Some(50),
      estimated_tokens_out: Some(10),
    };
    // in: 100 / 100 = 1s, out: (50+10)/(20)=3s -> total 4s
    assert_eq!(calculate_duration(&model, &cfg), 4);
  }
}