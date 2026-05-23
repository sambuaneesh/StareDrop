use anyhow::Result;

use crate::transfer_log::TransferLog;

pub fn export_json(log: &TransferLog) -> Result<String> {
    Ok(serde_json::to_string_pretty(log)?)
}
