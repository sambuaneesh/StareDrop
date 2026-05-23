use anyhow::Result;

pub fn compress_passthrough(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

pub fn decompress_passthrough(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}
