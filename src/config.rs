use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClassificationConfig {
    pub label: String,
    pub min_confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub classification_rate: Duration,
    pub classification_lock_list: Vec<ClassificationConfig>,
    pub classification_unlock_list: Vec<ClassificationConfig>,
    //
    pub logger_timezone: chrono::FixedOffset,
    pub unlock_grace_period: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            classification_rate: std::time::Duration::from_secs(3),
            logger_timezone: mountain_standard_time(),
            unlock_grace_period: Duration::from_secs(3),
            classification_lock_list: vec![ClassificationConfig {
                label: "cat".to_string(),
                min_confidence: 0.7,
            }],
            classification_unlock_list: vec![ClassificationConfig {
                label: "dog".to_string(),
                min_confidence: 0.7,
            }],
        }
    }
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
