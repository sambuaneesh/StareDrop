use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BinaryFrameType {
    Manifest = 1,
    Data = 2,
    Control = 3,
    End = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryFrameHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub frame_type: BinaryFrameType,
    pub flags: u16,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub payload_length: u32,
}

impl BinaryFrameHeader {
    pub const MAGIC: [u8; 4] = *b"STRD";
    pub const BYTE_LEN: usize = 4 + 1 + 1 + 2 + 4 + 4 + 4;

    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::BYTE_LEN);
        out.extend_from_slice(&self.magic);
        out.push(self.version);
        out.push(self.frame_type as u8);
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.chunk_index.to_le_bytes());
        out.extend_from_slice(&self.total_chunks.to_le_bytes());
        out.extend_from_slice(&self.payload_length.to_le_bytes());
        out
    }

    pub fn from_le_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::BYTE_LEN {
            bail!("buffer too short for binary header");
        }
        let mut magic = [0_u8; 4];
        magic.copy_from_slice(&buf[0..4]);
        let frame_type = match buf[5] {
            1 => BinaryFrameType::Manifest,
            2 => BinaryFrameType::Data,
            3 => BinaryFrameType::Control,
            4 => BinaryFrameType::End,
            other => bail!("invalid frame type {}", other),
        };
        Ok(Self {
            magic,
            version: buf[4],
            frame_type,
            flags: u16::from_le_bytes([buf[6], buf[7]]),
            chunk_index: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            total_chunks: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            payload_length: u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryFrameHeader, BinaryFrameType};

    #[test]
    fn header_round_trip() {
        let h = BinaryFrameHeader {
            magic: BinaryFrameHeader::MAGIC,
            version: 1,
            frame_type: BinaryFrameType::Data,
            flags: 7,
            chunk_index: 2,
            total_chunks: 8,
            payload_length: 42,
        };
        let bytes = h.to_le_bytes();
        let parsed = BinaryFrameHeader::from_le_bytes(&bytes).expect("parse");
        assert_eq!(parsed, h);
    }
}
