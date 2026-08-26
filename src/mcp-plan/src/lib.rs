//! mcp-plan - MCP server that provides planning tooling.

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

pub mod config;
pub mod connect;
pub mod db;
pub mod log;
pub mod migration;
pub mod models;
pub mod service;

#[cfg(test)]
pub mod test;

pub use migration::{MigrationRunner, Migrator};

use models::{NewTask, TaskStatus, TaskUpdate};
use rmcp::{
  Json, handler::server::wrapper::Parameters, model::ErrorData,
  schemars::JsonSchema, tool, tool_router,
};
use serde::Deserialize;
use service::Service;
use std::time::Instant;

/// Input for the `task` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskInput {
  pub id: Option<String>,
  pub link: Option<String>,
}

/// Input for the `children` tool.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ChildrenInput {
  pub parent_id: Option<String>,
}

/// Input for the `insert` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct InsertInput {
  pub object: NewTask,
  pub parent_id: Option<String>,
}

/// Output of the `insert` tool.
#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct InsertOutput {
  pub id: String,
}

/// Input for the `complete` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CompleteInput {
  pub task_id: String,
  pub status: String,
}

/// Input for the `fail` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct FailInput {
  pub task_id: String,
  pub status: String,
}

/// Input for the `escalate` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct EscalateInput {
  pub task_id: String,
}

/// Input for the `ready` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ReadyInput {
  pub task_id: String,
  pub object: TaskUpdate,
}

/// MCP server that provides planning tooling.
#[derive(Debug, Clone)]
pub struct PlanServer {
  pub service: Service,
}

#[tool_router(server_handler)]
impl PlanServer {
  /// Fetch a single task by id or link (exactly one required).
  #[tool(
    name = "task",
    description = "Fetch a single task by id or link (exactly one required)."
  )]
  pub async fn task(
    &self,
    Parameters(input): Parameters<TaskInput>,
  ) -> Result<Json<models::Task>, ErrorData> {
    let started = Instant::now();

    let reference = match (&input.id, &input.link) {
      (Some(_), Some(_)) => {
        tracing::warn!(
          tool = "task",
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `task` rejected: both id and link provided",
        );
        return Err(ErrorData::invalid_params(
          "provide exactly one of `id` or `link`, not both",
          None,
        ));
      }
      (None, None) => {
        tracing::warn!(
          tool = "task",
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `task` rejected: neither id nor link provided",
        );
        return Err(ErrorData::invalid_params(
          "provide exactly one of `id` or `link`",
          None,
        ));
      }
      (Some(id), None) => service::TaskRef::Id(id.to_owned()),
      (None, Some(link)) => service::TaskRef::Link(link.to_owned()),
    };
    let key = reference.as_str().to_owned();

    match self.service.task_ref(reference).await {
      Ok(Some(task)) => {
        tracing::info!(
          tool = "task",
          task_id = %key,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `task` completed",
        );
        Ok(Json(task))
      }
      Ok(None) => {
        tracing::warn!(
          tool = "task",
          task_id = %key,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "task not found",
        );
        Err(ErrorData::internal_error(
          format!("task `{key}` not found"),
          None,
        ))
      }
      Err(e) => {
        tracing::error!(
          tool = "task",
          task_id = %key,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `task` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Return all sources, ordered by id.
  #[tool(name = "sources", description = "Return all sources, ordered by id.")]
  pub async fn sources(&self) -> Result<Json<Vec<models::Source>>, ErrorData> {
    let started = Instant::now();
    match self.service.sources().await {
      Ok(list) => {
        tracing::debug!(
          tool = "sources",
          count = list.len(),
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `sources` completed",
        );
        Ok(Json(list))
      }
      Err(e) => {
        tracing::error!(
          tool = "sources",
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `sources` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Return the direct children of a task (`parent_id`), or roots when omitted.
  #[tool(
    name = "children",
    description = "Return the direct children of a task (`parent_id`), or roots when omitted."
  )]
  pub async fn children(
    &self,
    Parameters(input): Parameters<ChildrenInput>,
  ) -> Result<Json<Vec<models::TaskSummary>>, ErrorData> {
    let started = Instant::now();
    match self.service.children(input.parent_id.as_deref()).await {
      Ok(list) => {
        tracing::debug!(
          tool = "children",
          parent_id = %input.parent_id.as_deref().unwrap_or(""),
          count = list.len(),
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `children` completed",
        );
        Ok(Json(list))
      }
      Err(e) => {
        tracing::error!(
          tool = "children",
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `children` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Insert a new task (status `ready`); returns the new id.
  #[tool(
    name = "insert",
    description = "Insert a new task (status `ready`); returns the new id."
  )]
  pub async fn insert(
    &self,
    Parameters(input): Parameters<InsertInput>,
  ) -> Result<Json<InsertOutput>, ErrorData> {
    let started = Instant::now();
    match self
      .service
      .insert(input.object, input.parent_id.as_deref())
      .await
    {
      Ok(id) => {
        tracing::info!(
          tool = "insert",
          task_id = %id,
          parent_id = %input.parent_id.as_deref().unwrap_or(""),
          message = "tool `insert` completed",
        );
        tracing::debug!(
          tool = "insert",
          task_id = %id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `insert` completed",
        );
        Ok(Json(InsertOutput { id }))
      }
      Err(e) => {
        tracing::error!(
          tool = "insert",
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `insert` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Mark a task as `success` or `failure`.
  #[tool(
    name = "complete",
    description = "Mark a task as `success` or `failure`."
  )]
  pub async fn complete(
    &self,
    Parameters(input): Parameters<CompleteInput>,
  ) -> Result<Json<models::Task>, ErrorData> {
    let started = Instant::now();
    let status = match TaskStatus::from(input.status.as_str()) {
      TaskStatus::Success => TaskStatus::Success,
      TaskStatus::Failure => TaskStatus::Failure,
      _ => {
        tracing::warn!(
          tool = "complete",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `complete` rejected: invalid status",
        );
        return Err(ErrorData::invalid_params(
          "`complete` only accepts `success` or `failure`",
          None,
        ));
      }
    };
    match self.service.complete(&input.task_id, status).await {
      Ok(task) => {
        tracing::info!(
          tool = "complete",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `complete` completed",
        );
        Ok(Json(task))
      }
      Err(e) => {
        tracing::error!(
          tool = "complete",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `complete` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Mark a task as `failure`, incrementing `retries`. The `status` argument
  /// records the reason for the failure. Once retries reach the configured
  /// limit the task is escalated instead.
  #[tool(
    name = "fail",
    description = "Mark a task as `failure`, incrementing retries. The `status` argument records the reason for the failure."
  )]
  pub async fn fail(
    &self,
    Parameters(input): Parameters<FailInput>,
  ) -> Result<Json<models::Task>, ErrorData> {
    let started = Instant::now();
    if input.status.trim().is_empty() {
      tracing::warn!(
        tool = "fail",
        task_id = %input.task_id,
        duration_ms = started.elapsed().as_millis() as u64,
        message = "tool `fail` rejected: empty status",
      );
      return Err(ErrorData::invalid_params(
        "`status` must not be empty",
        None,
      ));
    }
    match self.service.fail(&input.task_id).await {
      Ok(task) => {
        tracing::info!(
          tool = "fail",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `fail` completed",
        );
        tracing::debug!(
          tool = "fail",
          task_id = %input.task_id,
          reason = %input.status,
          message = "task failed",
        );
        Ok(Json(task))
      }
      Err(e) => {
        tracing::error!(
          tool = "fail",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `fail` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Mark a task as `escalated` (blocked pending human review).
  #[tool(
    name = "escalate",
    description = "Mark a task as `escalated` (blocked pending human review)."
  )]
  pub async fn escalate(
    &self,
    Parameters(input): Parameters<EscalateInput>,
  ) -> Result<Json<models::Task>, ErrorData> {
    let started = Instant::now();
    match self.service.escalate(&input.task_id).await {
      Ok(task) => {
        tracing::info!(
          tool = "escalate",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `escalate` completed",
        );
        Ok(Json(task))
      }
      Err(e) => {
        tracing::error!(
          tool = "escalate",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `escalate` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Update mutable fields and set status back to `ready`.
  #[tool(
    name = "ready",
    description = "Update mutable fields and set status back to `ready`."
  )]
  pub async fn ready(
    &self,
    Parameters(input): Parameters<ReadyInput>,
  ) -> Result<Json<models::Task>, ErrorData> {
    let started = Instant::now();
    match self.service.ready(&input.task_id, input.object).await {
      Ok(task) => {
        tracing::info!(
          tool = "ready",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `ready` completed",
        );
        Ok(Json(task))
      }
      Err(e) => {
        tracing::error!(
          tool = "ready",
          task_id = %input.task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `ready` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Return tasks needing work, sorted by priority then planning→execution.
  #[tool(
    name = "queue",
    description = "Return tasks needing work, sorted by priority then planning→execution."
  )]
  pub async fn queue(
    &self,
  ) -> Result<Json<Vec<models::QueuedTask>>, ErrorData> {
    let started = Instant::now();
    match self.service.queue().await {
      Ok(list) => {
        tracing::debug!(
          tool = "queue",
          count = list.len(),
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `queue` completed",
        );
        Ok(Json(list))
      }
      Err(e) => {
        tracing::error!(
          tool = "queue",
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `queue` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }
}
