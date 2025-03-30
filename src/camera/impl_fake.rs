use crate::camera::interface::Camera;
use crate::logger::interface::Logger;

pub struct FakeCamera {
    logger: Box<dyn Logger>,
}

impl FakeCamera {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Self {
            logger: logger.with_namespace("camera.fake"),
        }
    }
}

impl Camera for FakeCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Starting fake camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Stopping fake camera...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.logger.info("Capturing fake frame...")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let image = vec![0; 100 * 100 * 3];
        Ok(image)
    }
}
