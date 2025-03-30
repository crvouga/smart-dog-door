use crate::logger::interface::Logger;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct LoggerConsole {
    namespace: Option<String>,
    timezone: chrono::FixedOffset,
}

impl LoggerConsole {
    pub fn new(timezone: chrono::FixedOffset) -> Self {
        Self {
            namespace: None,
            timezone,
        }
    }
}

impl Logger for LoggerConsole {
    fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let utc_now = Utc::now();
        let local_time = utc_now.with_timezone(&self.timezone);
        let formatted = local_time.format("%Y-%m-%d %I:%M:%S%.3f %p");
        match &self.namespace {
            Some(namespace) => println!("[{}] {}: {}", formatted, namespace, message),
            None => println!("[{}] {}", formatted, message),
        };
        Ok(())
    }

    fn with_namespace(&self, namespace: &str) -> Box<dyn Logger> {
        let new_namespace = match &self.namespace {
            Some(current) => format!("{}:{}", current, namespace),
            None => namespace.to_string(),
        };

        Box::new(LoggerConsole {
            namespace: Some(new_namespace),
            timezone: self.timezone,
        })
    }
}
