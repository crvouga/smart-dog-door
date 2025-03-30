pub trait Camera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}
