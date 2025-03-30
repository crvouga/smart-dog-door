#[derive(Debug, Clone)]
pub enum DeviceCameraEvent {
    Disconnected,
    Connected,
}

pub trait DeviceCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
    fn events(&self) -> std::sync::mpsc::Receiver<DeviceCameraEvent>;
}
