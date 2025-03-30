use crate::device_camera::interface::DeviceCamera;
use crate::logger::interface::Logger;

pub struct DeviceCameraFake {
    logger: Box<dyn Logger>,
}

impl DeviceCameraFake {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Self {
            logger: logger.with_namespace("camera").with_namespace("fake"),
        }
    }
}

impl DeviceCamera for DeviceCameraFake {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Starting camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Camera started")?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Stopping camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Camera stopped")?;
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.logger.info("Capturing frame...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let image = vec![0; 100 * 100 * 3];
        self.logger.info("Frame captured")?;
        Ok(image)
    }

    fn events(&self) -> std::sync::mpsc::Sender<CameraEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            tx.send(CameraEvent::Connected).unwrap();
        });
        tx
    }
}
