use crate::logger::interface::Logger;

#[derive(Debug, Clone)]
pub struct ConsoleLogger {
    namespace: String,
}

impl ConsoleLogger {
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }
}

impl Logger for ConsoleLogger {
    fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}: {}", self.namespace, message);
        Ok(())
    }

    fn with_namespace(&self, namespace: &str) -> Box<dyn Logger> {
        Box::new(ConsoleLogger::new(format!(
            "{}:{}",
            self.namespace, namespace
        )))
    }
}
