use std::fmt::{self, Debug, Formatter};

#[derive(Debug, Clone)]
pub enum DeviceCameraEvent {
    Disconnected,
    Connected,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Frame(pub Vec<u8>);

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Frame")
    }
}

pub trait DeviceCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    #[allow(dead_code)]
    fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn capture_frame(&self) -> Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>;
    fn events(&self) -> std::sync::mpsc::Receiver<DeviceCameraEvent>;
}
