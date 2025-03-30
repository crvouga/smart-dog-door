use crate::camera::interface::Camera;

pub struct FakeCamera {}

impl FakeCamera {
    pub fn new() -> Self {
        Self {}
    }
}

impl Camera for FakeCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting fake camera...");
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Stopping fake camera...");
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        println!("Capturing fake frame...");
        let image = vec![0; 100 * 100 * 3];
        Ok(image)
    }
}
