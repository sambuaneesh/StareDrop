use anyhow::{Result, bail};

pub fn derive_key_stub(_password: &str) -> Result<[u8; 32]> {
    bail!("argon2id KDF is planned for Phase 5")
}
