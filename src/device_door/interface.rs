pub trait DeviceDoor {
    fn open(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    #[allow(dead_code)]
    fn is_open(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    fn events(&self) -> std::sync::mpsc::Receiver<DeviceDoorEvent>;
}

#[derive(Debug, Clone)]
pub enum DeviceDoorEvent {
    Connected,
    Disconnected,
}
