use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub classification_rate: Duration,
    pub classification_min_confidence_dog: f32,
    pub classification_min_confidence_cat: f32,
    pub logger_timezone: chrono::FixedOffset,
    pub unlock_grace_period: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            classification_rate: std::time::Duration::from_secs(3),
            classification_min_confidence_dog: 0.7,
            classification_min_confidence_cat: 0.7,
            logger_timezone: mountain_standard_time(),
            unlock_grace_period: Duration::from_secs(5),
        }
    }
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
