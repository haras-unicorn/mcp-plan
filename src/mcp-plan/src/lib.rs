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

use rmcp::tool_router;

pub mod config;
pub mod connect;
pub mod db;
pub mod log;
pub mod migration;
pub mod models;
pub mod service;

pub use migration::{MigrationRunner, Migrator};

use models::{NewTask, TaskStatus, TaskUpdate};
use rmcp::model::{CallToolResult, ErrorData};
use rmcp::serde_json::json;
use service::Service;

/// MCP server that provides planning tooling.
#[derive(Debug, Clone)]
pub struct PlanServer {
  pub service: Service,
}

impl PlanServer {
  /// Convert a positive result into a structured-content result.
  fn structured<T: serde::Serialize>(
    &self,
    value: T,
  ) -> Result<CallToolResult, ErrorData> {
    rmcp::serde_json::to_value(value)
      .map(CallToolResult::structured)
      .map_err(|e| ErrorData::internal_error(e.to_string(), None))
  }
}

#[tool_router(server_handler)]
impl PlanServer {
  /// Fetch a single task by id.
  pub async fn task(&self, id: String) -> Result<CallToolResult, ErrorData> {
    match self.service.task(&id).await {
      Ok(Some(task)) => self.structured(&task),
      Ok(None) => Ok(CallToolResult::structured(
        json!({ "error": format!("task `{id}` not found") }),
      )),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Return the direct children of a task (`parent_id`), or roots when omitted.
  pub async fn children(
    &self,
    parent_id: Option<String>,
  ) -> Result<CallToolResult, ErrorData> {
    match self.service.children(parent_id.as_deref()).await {
      Ok(list) => self.structured(&list),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Insert a new task (status `ready`); returns the new id.
  pub async fn insert(
    &self,
    object: NewTask,
    parent_id: Option<String>,
  ) -> Result<CallToolResult, ErrorData> {
    match self.service.insert(object, parent_id.as_deref()).await {
      Ok(id) => self.structured(json!({ "id": id })),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Mark a task as `success` or `failure`.
  pub async fn complete(
    &self,
    task_id: String,
    status: String,
  ) -> Result<CallToolResult, ErrorData> {
    let status = match TaskStatus::from(status.as_str()) {
      TaskStatus::Success => TaskStatus::Success,
      TaskStatus::Failure => TaskStatus::Failure,
      _ => {
        return Err(ErrorData::invalid_params(
          "`complete` only accepts `success` or `failure`",
          None,
        ));
      }
    };
    match self.service.complete(&task_id, status).await {
      Ok(task) => self.structured(&task),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Mark a task as `failure`, incrementing `retries`. The `status` argument
  /// records the reason for the failure. Once retries reach the configured
  /// limit the task is escalated instead.
  pub async fn fail(
    &self,
    task_id: String,
    status: String,
  ) -> Result<CallToolResult, ErrorData> {
    if status.trim().is_empty() {
      return Err(ErrorData::invalid_params(
        "`status` must not be empty",
        None,
      ));
    }
    tracing::info!(task_id, reason = status, "task failed");
    match self.service.fail(&task_id).await {
      Ok(task) => self.structured(&task),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Mark a task as `escalated` (blocked pending human review).
  pub async fn escalate(
    &self,
    task_id: String,
  ) -> Result<CallToolResult, ErrorData> {
    match self.service.escalate(&task_id).await {
      Ok(task) => self.structured(&task),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Update mutable fields and set status back to `ready`.
  pub async fn ready(
    &self,
    task_id: String,
    object: TaskUpdate,
  ) -> Result<CallToolResult, ErrorData> {
    match self.service.ready(&task_id, object).await {
      Ok(task) => self.structured(&task),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }

  /// Return tasks needing work, sorted by priority then planning→execution.
  pub async fn queue(&self) -> Result<CallToolResult, ErrorData> {
    match self.service.queue().await {
      Ok(list) => self.structured(&list),
      Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
  }
}