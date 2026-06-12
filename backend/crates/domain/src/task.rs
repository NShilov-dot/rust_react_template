use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTitle(String);

impl TaskTitle {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 200 {
            return Err(DomainError::InvalidTaskTitle(raw));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDescription(String);

impl TaskDescription {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 5_000 {
            return Err(DomainError::InvalidTaskDescription(raw));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            other => Err(DomainError::InvalidTaskStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(DomainError::InvalidTaskPriority(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub owner_id: UserId,
    pub title: TaskTitle,
    pub description: Option<TaskDescription>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(
        owner_id: UserId,
        title: TaskTitle,
        description: Option<TaskDescription>,
        priority: TaskPriority,
        due_date: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            owner_id,
            title,
            description,
            status: TaskStatus::Todo,
            priority,
            due_date,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_rejects_blank() {
        assert!(TaskTitle::parse("   ").is_err());
    }

    #[test]
    fn title_rejects_oversize() {
        let long = "a".repeat(201);
        assert!(TaskTitle::parse(long).is_err());
    }

    #[test]
    fn title_trims_input() {
        let t = TaskTitle::parse("  hi  ").unwrap();
        assert_eq!(t.as_str(), "hi");
    }

    #[test]
    fn status_roundtrip() {
        for s in [TaskStatus::Todo, TaskStatus::InProgress, TaskStatus::Done] {
            assert_eq!(TaskStatus::parse(s.as_str()).unwrap(), s);
        }
    }

    #[test]
    fn status_rejects_garbage() {
        assert!(TaskStatus::parse("zombie").is_err());
    }

    #[test]
    fn priority_roundtrip() {
        for p in [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High] {
            assert_eq!(TaskPriority::parse(p.as_str()).unwrap(), p);
        }
    }

    #[test]
    fn new_task_defaults_to_todo() {
        let title = TaskTitle::parse("write tests").unwrap();
        let task = Task::new(UserId::new(), title, None, TaskPriority::Medium, None);
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.description.is_none());
    }
}
