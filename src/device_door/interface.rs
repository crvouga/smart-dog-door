pub trait DeviceDoor {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn unlock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_unlocked(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    fn events(&self) -> std::sync::mpsc::Receiver<DeviceDoorEvent>;
}

#[derive(Debug, Clone)]
pub enum DeviceDoorEvent {
    Connected,
    Disconnected,
}
