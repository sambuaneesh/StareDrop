use anyhow::{Result, bail};

pub fn encode_stub(_payload: &[u8]) -> Result<()> {
    bail!("custom grid codec is not implemented in Phase 1")
}
