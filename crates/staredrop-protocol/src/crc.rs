pub fn crc32(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn crc_is_stable() {
        assert_eq!(crc32(b"hello"), 0x3610_a686);
    }
}
