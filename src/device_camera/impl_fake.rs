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

    fn capture_frame(&self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Capturing frame...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let image = vec![0; 100 * 100 * 3];
        self.logger.info("Frame captured")?;
        Ok(vec![image])
    }

    fn events(&self) -> std::sync::mpsc::Receiver<DeviceCameraEvent> {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            tx.send(DeviceCameraEvent::Connected).unwrap();

            loop {
                std::thread::sleep(std::time::Duration::from_secs(300)); // Sleep for 5 minutes

                // 0.1% chance of disconnecting
                if rand::random::<f32>() < 0.001 {
                    tx.send(DeviceCameraEvent::Disconnected).unwrap();
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    tx.send(DeviceCameraEvent::Connected).unwrap();
                }
            }
        });

        rx
    }
}
