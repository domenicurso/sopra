use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub cwd: Option<String>,
    pub exit_status: Option<i32>,
    pub duration_ms: Option<u64>,
    pub timestamp: Option<i64>,
}
