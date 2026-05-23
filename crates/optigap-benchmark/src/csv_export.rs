use anyhow::Result;

use crate::transfer_log::TransferLog;

pub fn export_csv_line(log: &TransferLog) -> Result<String> {
    Ok(format!(
        "{},{},{},{},{}",
        log.session_id, log.file_name, log.file_size, log.started_at, log.ended_at
    ))
}
