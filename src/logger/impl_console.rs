use crate::logger::interface::Logger;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ConsoleLogger {
    namespace: String,
    timezone: chrono::FixedOffset,
}

impl ConsoleLogger {
    pub fn new(namespace: String, timezone: chrono::FixedOffset) -> Self {
        Self {
            namespace,
            timezone,
        }
    }

    pub fn with_timezone(namespace: String, timezone: chrono::FixedOffset) -> Self {
        Self {
            namespace,
            timezone,
        }
    }
}

impl Logger for ConsoleLogger {
    fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let utc_now = Utc::now();
        let local_time = utc_now.with_timezone(&self.timezone);
        let formatted = local_time.format("%Y-%m-%d %I:%M:%S%.3f %p");
        println!("[{}] {}: {}", formatted, self.namespace, message);
        Ok(())
    }

    fn with_namespace(&self, namespace: &str) -> Box<dyn Logger> {
        Box::new(ConsoleLogger {
            namespace: format!("{}:{}", self.namespace, namespace),
            timezone: self.timezone,
        })
    }
}
