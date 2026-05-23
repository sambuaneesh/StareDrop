use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub file_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl SessionInfo {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SessionInfo;

    #[test]
    fn creates_distinct_ids() {
        let a = SessionInfo::new();
        let b = SessionInfo::new();
        assert_ne!(a.session_id, b.session_id);
        assert_ne!(a.file_id, b.file_id);
    }
}
