use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
use crate::library::logger::interface::Logger;
use std::sync::Arc;

pub struct DeviceCameraFake {
    logger: Arc<dyn Logger>,
}

impl DeviceCameraFake {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger: logger.with_namespace("camera").with_namespace("fake"),
        }
    }
}

impl DeviceCamera for DeviceCameraFake {
    fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Camera started")?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Stopping camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Camera stopped")?;
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Capturing frame...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let image = vec![0; 100 * 100 * 3];
        self.logger.info("Frame captured")?;
        Ok(image)
    }

    fn events(&self) -> std::sync::mpsc::Receiver<DeviceCameraEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            tx_clone.send(DeviceCameraEvent::Connected).unwrap();
        });
        rx
    }
}
