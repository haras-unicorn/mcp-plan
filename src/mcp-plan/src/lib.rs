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

#[cfg(test)]
pub mod test;

pub use migration::{MigrationRunner, Migrator};

use models::{NewTask, TaskStatus, TaskUpdate};
use rmcp::model::{CallToolResult, ErrorData};
use rmcp::serde_json::json;
use service::Service;
use std::time::Instant;

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
    let started = Instant::now();
    match self.service.task(&id).await {
      Ok(Some(task)) => {
        tracing::info!(
          tool = "task",
          task_id = %id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `task` completed",
        );
        self.structured(&task)
      }
      Ok(None) => {
        tracing::warn!(
          tool = "task",
          task_id = %id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "task not found",
        );
        Ok(CallToolResult::structured(
          json!({ "error": format!("task `{id}` not found") }),
        ))
      }
      Err(e) => {
        tracing::error!(
          tool = "task",
          task_id = %id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `task` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Return the direct children of a task (`parent_id`), or roots when omitted.
  pub async fn children(
    &self,
    parent_id: Option<String>,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    match self.service.children(parent_id.as_deref()).await {
      Ok(list) => {
        tracing::debug!(
          tool = "children",
          parent_id = %parent_id.as_deref().unwrap_or(""),
          count = list.len(),
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `children` completed",
        );
        self.structured(&list)
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
  pub async fn insert(
    &self,
    object: NewTask,
    parent_id: Option<String>,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    match self.service.insert(object, parent_id.as_deref()).await {
      Ok(id) => {
        tracing::info!(
          tool = "insert",
          task_id = %id,
          parent_id = %parent_id.as_deref().unwrap_or(""),
          message = "tool `insert` completed",
        );
        tracing::debug!(
          tool = "insert",
          task_id = %id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `insert` completed",
        );
        self.structured(json!({ "id": id }))
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
  pub async fn complete(
    &self,
    task_id: String,
    status: String,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    let status = match TaskStatus::from(status.as_str()) {
      TaskStatus::Success => TaskStatus::Success,
      TaskStatus::Failure => TaskStatus::Failure,
      _ => {
        tracing::warn!(
          tool = "complete",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `complete` rejected: invalid status",
        );
        return Err(ErrorData::invalid_params(
          "`complete` only accepts `success` or `failure`",
          None,
        ));
      }
    };
    match self.service.complete(&task_id, status).await {
      Ok(task) => {
        tracing::info!(
          tool = "complete",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `complete` completed",
        );
        self.structured(&task)
      }
      Err(e) => {
        tracing::error!(
          tool = "complete",
          task_id = %task_id,
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
  pub async fn fail(
    &self,
    task_id: String,
    status: String,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    if status.trim().is_empty() {
      tracing::warn!(
        tool = "fail",
        task_id = %task_id,
        duration_ms = started.elapsed().as_millis() as u64,
        message = "tool `fail` rejected: empty status",
      );
      return Err(ErrorData::invalid_params(
        "`status` must not be empty",
        None,
      ));
    }
    match self.service.fail(&task_id).await {
      Ok(task) => {
        tracing::info!(
          tool = "fail",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `fail` completed",
        );
        tracing::debug!(
          tool = "fail",
          task_id = %task_id,
          reason = %status,
          message = "task failed",
        );
        self.structured(&task)
      }
      Err(e) => {
        tracing::error!(
          tool = "fail",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `fail` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Mark a task as `escalated` (blocked pending human review).
  pub async fn escalate(
    &self,
    task_id: String,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    match self.service.escalate(&task_id).await {
      Ok(task) => {
        tracing::info!(
          tool = "escalate",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `escalate` completed",
        );
        self.structured(&task)
      }
      Err(e) => {
        tracing::error!(
          tool = "escalate",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `escalate` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Update mutable fields and set status back to `ready`.
  pub async fn ready(
    &self,
    task_id: String,
    object: TaskUpdate,
  ) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    match self.service.ready(&task_id, object).await {
      Ok(task) => {
        tracing::info!(
          tool = "ready",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `ready` completed",
        );
        self.structured(&task)
      }
      Err(e) => {
        tracing::error!(
          tool = "ready",
          task_id = %task_id,
          duration_ms = started.elapsed().as_millis() as u64,
          error = %e,
          message = "tool `ready` failed",
        );
        Err(ErrorData::internal_error(e.to_string(), None))
      }
    }
  }

  /// Return tasks needing work, sorted by priority then planning→execution.
  pub async fn queue(&self) -> Result<CallToolResult, ErrorData> {
    let started = Instant::now();
    match self.service.queue().await {
      Ok(list) => {
        tracing::debug!(
          tool = "queue",
          count = list.len(),
          duration_ms = started.elapsed().as_millis() as u64,
          message = "tool `queue` completed",
        );
        self.structured(&list)
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
