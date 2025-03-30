use std::sync::Arc;

pub trait Logger: Send + Sync {
    fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn with_namespace(&self, namespace: &str) -> Arc<dyn Logger + Send + Sync>;
}
