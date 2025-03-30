use std::time::Duration;

pub struct Config {
    pub classification_rate: Duration,
    pub classification_min_confidence_dog: f32,
    pub classification_min_confidence_cat: f32,
    pub timezone: chrono::FixedOffset,
}
