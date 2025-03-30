pub trait DeviceDoor {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn unlock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_unlocked(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    fn events(&self) -> std::sync::mpsc::Sender<DoorEvent>;
}

pub enum DoorEvent {
    Connected,
    Disconnected,
}
