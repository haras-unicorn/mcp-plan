//! DTOs for the MCP tool surface. These are plain serde types (no sea-orm)
//! so the wire API is stable and independent of the persistence schema. The
//! database schema uses the matching sea-orm active enums in `crate::db`;
//! conversions between the two live at the bottom of this module.

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

/// The full task object returned by multiple tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskOutput {
  pub id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent_id: Option<String>,
  pub source_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link: Option<String>,
  pub title: String,
  pub description: String,
  pub status: TaskStatus,
  pub priority: TaskPriority,
  #[serde(default)]
  pub retries: i32,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub estimated_tokens_in: i64,
  pub estimated_tokens_reasoning: i64,
  pub estimated_tokens_out: i64,
}

/// A source as returned by `sources` tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SourceOutput {
  pub id: String,
  pub title: String,
  pub description: String,
  #[serde(rename = "type")]
  #[schemars(rename = "type")]
  pub source_type: String,
}

/// A compact representation of a task, used by `children` tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ChildrenTaskOutput {
  pub id: String,
  pub title: String,
  pub status: TaskStatus,
  pub priority: TaskPriority,
  pub estimated_tokens_in: i64,
  pub estimated_tokens_reasoning: i64,
  pub estimated_tokens_out: i64,
}

/// A task returned by `queue` tool, annotated with its server-decided workload
/// type and token-based duration in seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct QueueTaskOutput {
  pub id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent_id: Option<String>,
  pub source_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link: Option<String>,
  pub title: String,
  pub description: String,
  pub status: TaskStatus,
  pub priority: TaskPriority,
  #[serde(default)]
  pub retries: i32,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub estimated_tokens_in: i64,
  pub estimated_tokens_reasoning: i64,
  pub estimated_tokens_out: i64,
  pub kind: TaskType,
  pub duration_secs: u64,
}

/// Input for `ready` tool.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ReadyInput {
  pub id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub priority: Option<TaskPriority>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_in: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_reasoning: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub estimated_tokens_out: Option<i64>,
}

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

/// Input for the `source` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SourceInput {
  pub id: String,
}

/// Input for the `insert` tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct InsertInput {
  pub title: String,
  pub description: String,
  pub source_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parent_id: Option<String>,
  #[serde(default)]
  pub priority: TaskPriority,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link: Option<String>,
  pub estimated_tokens_in: i64,
  pub estimated_tokens_reasoning: i64,
  pub estimated_tokens_out: i64,
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

/// Parse a wire status string, rejecting unknown values.
impl TryFrom<&str> for TaskStatus {
  type Error = String;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "ready" => Ok(Self::Ready),
      "success" => Ok(Self::Success),
      "failure" => Ok(Self::Failure),
      "escalated" => Ok(Self::Escalated),
      other => Err(format!("unknown task status `{other}`")),
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

// Mappings between the wire DTO enums and the sea-orm active enums used by the
// persistence layer (`crate::db::entities::tasks`).

impl From<tasks::TaskStatus> for TaskStatus {
  fn from(value: tasks::TaskStatus) -> Self {
    match value {
      tasks::TaskStatus::Ready => Self::Ready,
      tasks::TaskStatus::Success => Self::Success,
      tasks::TaskStatus::Failure => Self::Failure,
      tasks::TaskStatus::Escalated => Self::Escalated,
    }
  }
}

impl From<TaskStatus> for tasks::TaskStatus {
  fn from(value: TaskStatus) -> Self {
    match value {
      TaskStatus::Ready => Self::Ready,
      TaskStatus::Success => Self::Success,
      TaskStatus::Failure => Self::Failure,
      TaskStatus::Escalated => Self::Escalated,
    }
  }
}

impl From<tasks::TaskPriority> for TaskPriority {
  fn from(value: tasks::TaskPriority) -> Self {
    match value {
      tasks::TaskPriority::Critical => Self::Critical,
      tasks::TaskPriority::High => Self::High,
      tasks::TaskPriority::Medium => Self::Medium,
      tasks::TaskPriority::Low => Self::Low,
    }
  }
}

impl From<TaskPriority> for tasks::TaskPriority {
  fn from(value: TaskPriority) -> Self {
    match value {
      TaskPriority::Critical => Self::Critical,
      TaskPriority::High => Self::High,
      TaskPriority::Medium => Self::Medium,
      TaskPriority::Low => Self::Low,
    }
  }
}

impl From<tasks::Model> for TaskOutput {
  fn from(model: tasks::Model) -> Self {
    Self {
      id: model.id,
      parent_id: model.parent_id,
      source_id: model.source_id,
      link: model.link,
      title: model.title,
      description: model.description,
      status: TaskStatus::from(model.status),
      priority: TaskPriority::from(model.priority),
      retries: model.retries,
      created_at: model.created_at,
      updated_at: model.updated_at,
      estimated_tokens_in: model.estimated_tokens_in,
      estimated_tokens_reasoning: model.estimated_tokens_reasoning,
      estimated_tokens_out: model.estimated_tokens_out,
    }
  }
}

impl From<&tasks::Model> for ChildrenTaskOutput {
  fn from(model: &tasks::Model) -> Self {
    Self {
      id: model.id.clone(),
      title: model.title.clone(),
      status: TaskStatus::from(model.status),
      priority: TaskPriority::from(model.priority),
      estimated_tokens_in: model.estimated_tokens_in,
      estimated_tokens_reasoning: model.estimated_tokens_reasoning,
      estimated_tokens_out: model.estimated_tokens_out,
    }
  }
}

impl From<sources::Model> for SourceOutput {
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
    let task = TaskOutput {
      id: "t1".to_owned(),
      parent_id: Some("p".to_owned()),
      source_id: "d".to_owned(),
      link: None,
      title: "T".to_owned(),
      description: "D".to_owned(),
      status: TaskStatus::Ready,
      priority: TaskPriority::Medium,
      retries: 0,
      created_at: chrono::Utc::now(),
      updated_at: chrono::Utc::now(),
      estimated_tokens_in: 100,
      estimated_tokens_reasoning: 100,
      estimated_tokens_out: 100,
    };
    let json = rmcp::serde_json::to_value(&task)?;
    let back: TaskOutput = rmcp::serde_json::from_value(json)?;
    assert_eq!(back, task);
    Ok(())
  }
}
