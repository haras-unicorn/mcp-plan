//! DTOs for the MCP tool surface. These are plain serde types (no sea-orm)
//! so the wire API is stable and independent of the persistence schema.

use crate::db::entities::{sources, tasks};
use chrono::{DateTime, Utc};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Lifecycle status of a task.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum TaskStatus {
  Ready,
  Success,
  Failure,
  Escalated,
}

/// Priority of a task.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum TaskPriority {
  Critical,
  High,
  #[default]
  Medium,
  Low,
}

/// Work classification decided by the server based on token estimates.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(crate = "rmcp::schemars")]
pub enum TaskType {
  Planning,
  Execution,
}

/// The full task object returned by `task()` and used by `insert()`/`ready()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct Task {
  pub id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source_id: Option<String>,
  pub title: String,
  pub description: String,
  pub status: TaskStatus,
  pub priority: TaskPriority,
  #[serde(default)]
  pub retries: i32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<DateTime<Utc>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<DateTime<Utc>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_in: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_reasoning: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_out: Option<i64>,
}

/// A source as returned by `sources()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct Source {
  pub id: String,
  pub title: String,
  pub description: String,
  #[serde(rename = "type")]
  #[schemars(rename = "type")]
  pub source_type: String,
}

/// A compact representation of a task, used by `children()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskSummary {
  pub id: String,
  pub title: String,
  pub status: TaskStatus,
  pub priority: TaskPriority,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_in: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_reasoning: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_out: Option<i64>,
}

/// A task returned by `queue()`, annotated with its server-decided workload
/// type and token-based duration in seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct QueuedTask {
  pub task: Task,
  pub kind: TaskType,
  pub duration_secs: u64,
}

/// Input for creating a new task.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct NewTask {
  pub title: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub priority: TaskPriority,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_in: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_reasoning: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_out: Option<i64>,
}

/// Mutable fields that `ready()` accepts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskUpdate {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub priority: Option<TaskPriority>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_in: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_reasoning: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_out: Option<i64>,
}

impl TaskStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Ready => "ready",
      Self::Success => "success",
      Self::Failure => "failure",
      Self::Escalated => "escalated",
    }
  }
}

impl TaskPriority {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Critical => "critical",
      Self::High => "high",
      Self::Medium => "medium",
      Self::Low => "low",
    }
  }
}

impl From<&str> for TaskStatus {
  fn from(value: &str) -> Self {
    match value {
      "success" => Self::Success,
      "failure" => Self::Failure,
      "escalated" => Self::Escalated,
      _ => Self::Ready,
    }
  }
}

impl From<&str> for TaskPriority {
  fn from(value: &str) -> Self {
    match value {
      "critical" => Self::Critical,
      "high" => Self::High,
      "low" => Self::Low,
      _ => Self::Medium,
    }
  }
}

impl From<tasks::Model> for Task {
  fn from(model: tasks::Model) -> Self {
    Self {
      id: model.id,
      parent_id: model.parent_id,
      source_id: model.source_id,
      title: model.title,
      description: model.description,
      status: TaskStatus::from(model.status.as_str()),
      priority: TaskPriority::from(model.priority.as_str()),
      retries: model.retries,
      created_at: model.created_at,
      updated_at: model.updated_at,
      estimated_tokens_in: model.estimated_tokens_in,
      estimated_tokens_reasoning: model.estimated_tokens_reasoning,
      estimated_tokens_out: model.estimated_tokens_out,
    }
  }
}

impl From<&tasks::Model> for TaskSummary {
  fn from(model: &tasks::Model) -> Self {
    Self {
      id: model.id.clone(),
      title: model.title.clone(),
      status: TaskStatus::from(model.status.as_str()),
      priority: TaskPriority::from(model.priority.as_str()),
      estimated_tokens_in: model.estimated_tokens_in,
      estimated_tokens_reasoning: model.estimated_tokens_reasoning,
      estimated_tokens_out: model.estimated_tokens_out,
    }
  }
}

impl From<sources::Model> for Source {
  fn from(model: sources::Model) -> Self {
    Self {
      id: model.id,
      title: model.title,
      description: model.description,
      source_type: model.source_type,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn statuses_are_lowercase() -> Result<(), Box<dyn std::error::Error>> {
    assert!(TaskStatus::Success.as_str() == "success");
    let parsed: TaskStatus = rmcp::serde_json::from_str("\"failure\"")?;
    assert_eq!(parsed, TaskStatus::Failure);
    let parsed: TaskStatus = rmcp::serde_json::from_str("\"ready\"")?;
    assert_eq!(parsed, TaskStatus::Ready);
    Ok(())
  }

  #[test]
  fn priority_rankings() -> Result<(), Box<dyn std::error::Error>> {
    assert!(TaskPriority::High.as_str() == "high");
    let parsed: TaskPriority = rmcp::serde_json::from_str("\"critical\"")?;
    assert_eq!(parsed, TaskPriority::Critical);
    Ok(())
  }

  #[test]
  fn task_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let task = Task {
      id: "t1".to_owned(),
      parent_id: Some("p".to_owned()),
      source_id: None,
      title: "T".to_owned(),
      description: "D".to_owned(),
      status: TaskStatus::Ready,
      priority: TaskPriority::Medium,
      retries: 0,
      created_at: None,
      updated_at: None,
      estimated_tokens_in: Some(100),
      estimated_tokens_reasoning: None,
      estimated_tokens_out: None,
    };
    let json = rmcp::serde_json::to_value(&task)?;
    let back: Task = rmcp::serde_json::from_value(json)?;
    assert_eq!(back, task);
    Ok(())
  }
}
