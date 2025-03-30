use crate::logger::interface::Logger;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ConsoleLogger {
    namespace: Option<String>,
    timezone: chrono::FixedOffset,
}

impl ConsoleLogger {
    pub fn new(timezone: chrono::FixedOffset) -> Self {
        Self {
            namespace: None,
            timezone,
        }
    }
}

impl Logger for ConsoleLogger {
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

        Box::new(ConsoleLogger {
            namespace: Some(new_namespace),
            timezone: self.timezone,
        })
    }

    fn clone(&self) -> Box<dyn Logger> {
        Box::new(ConsoleLogger {
            namespace: self.namespace.clone(),
            timezone: self.timezone,
        })
    }
}
