use std::time::Duration;

pub fn throughput_kbps(bytes: usize, elapsed: Duration) -> f64 {
    if elapsed.is_zero() {
        return 0.0;
    }
    (bytes as f64 / 1024.0) / elapsed.as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::throughput_kbps;
    use std::time::Duration;

    #[test]
    fn throughput_calculation() {
        let t = throughput_kbps(2048, Duration::from_secs(2));
        assert!((t - 1.0).abs() < 1e-9);
    }
}
