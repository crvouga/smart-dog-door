use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClassificationConfig {
    pub label: String,
    pub min_confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub tick_rate: Duration,
    pub camera_process_rate: Duration,
    pub lock_list: Vec<ClassificationConfig>,
    pub unlock_list: Vec<ClassificationConfig>,
    pub logger_timezone: chrono::FixedOffset,
    pub minimal_duration_unlocking: Duration,
    pub minimal_duration_locking: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick_rate: std::time::Duration::from_secs(1),
            camera_process_rate: std::time::Duration::from_secs(5),
            logger_timezone: mountain_standard_time(),
            minimal_duration_unlocking: Duration::from_secs(3),
            minimal_duration_locking: Duration::from_secs(3),
            lock_list: vec![ClassificationConfig {
                label: "cat".to_string(),
                min_confidence: 0.5,
            }],
            unlock_list: vec![ClassificationConfig {
                label: "dog".to_string(),
                min_confidence: 0.5,
            }],
        }
    }
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
