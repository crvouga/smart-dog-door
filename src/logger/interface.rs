pub trait Logger {
    fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn with_namespace(&self, namespace: &str) -> Box<dyn Logger>;
    fn clone(&self) -> Box<dyn Logger>;
}
