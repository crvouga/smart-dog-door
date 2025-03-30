pub enum DeviceCameraEvent {
    Disconnected,
    Connected,
}

pub trait DeviceCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn events(&self) -> std::sync::mpsc::Sender<DeviceCameraEvent>;
}
