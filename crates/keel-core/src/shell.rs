use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShellSnapshot {
    pub cwd: String,
    pub last_status: i32,
    pub last_duration_ms: Option<u64>,
    pub aliases: Vec<String>,
    pub functions: Vec<String>,
    pub environment: Vec<(String, String)>,
}
