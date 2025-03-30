use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClassificationConfig {
    pub label: String,
    pub min_confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub tick_rate: Duration,
    pub analyze_rate: Duration,
    pub lock_list: Vec<ClassificationConfig>,
    pub unlock_list: Vec<ClassificationConfig>,
    pub logger_timezone: chrono::FixedOffset,
    pub unlock_grace_period: Duration,
    pub locking_grace_period: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick_rate: std::time::Duration::from_secs(1),
            analyze_rate: std::time::Duration::from_secs(1),
            logger_timezone: mountain_standard_time(),
            unlock_grace_period: Duration::from_secs(3),
            locking_grace_period: Duration::from_secs(3),
            lock_list: vec![
                ClassificationConfig {
                    label: "cat".to_string(),
                    min_confidence: 0.5,
                },
                // ClassificationConfig {
                //     label: "turtle".to_string(),
                //     min_confidence: 0.7,
                // },
            ],
            unlock_list: vec![
                ClassificationConfig {
                    label: "dog".to_string(),
                    min_confidence: 0.8,
                },
                // ClassificationConfig {
                //     label: "person".to_string(),
                //     min_confidence: 0.7,
                // },
            ],
        }
    }
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
