use crate::camera::interface::Camera;
use rascam::{Config as CameraConfig, SimpleCamera};
use std::time::Duration;

pub struct RaspberryPiCameraConfig {
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub framerate: u32,
    pub exposure_mode: String,
    pub sensor_mode: u8,
    pub warmup_delay_secs: u64,
}

impl Default for RaspberryPiCameraConfig {
    fn default() -> Self {
        Self {
            resolution_width: 1920,
            resolution_height: 1080,
            framerate: 30,
            exposure_mode: "auto".to_string(),
            sensor_mode: 7, // Mode 7 is optimal for NoIR night vision
            warmup_delay_secs: 2,
        }
    }
}

pub struct CameraRaspberryPi {
    camera: SimpleCamera,
    config: RaspberryPiCameraConfig,
    logger: Box<dyn crate::logger::interface::Logger>,
}

impl CameraRaspberryPi {
    pub fn new(
        config: RaspberryPiCameraConfig,
        logger: Box<dyn crate::logger::interface::Logger>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut camera_config = RaspberryPiCameraConfig::new();
        camera_config.set_resolution(config.resolution_width, config.resolution_height);
        camera_config.set_framerate(config.framerate);
        camera_config.set_exposure_mode(&config.exposure_mode);
        camera_config.set_sensor_mode(config.sensor_mode);

        let camera = SimpleCamera::new(camera_config)?;

        Ok(Self {
            camera,
            config,
            logger: logger.with_namespace("raspberry_pi_camera"),
        })
    }
}

impl Camera for CameraRaspberryPi {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Initializing Raspberry Pi camera")?;
        self.camera.start_capture()?;
        // Wait for camera to warm up
        std::thread::sleep(Duration::from_secs(self.config.warmup_delay_secs));
        self.logger.info("Camera initialized successfully")?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Stopping camera capture")?;
        self.camera.stop_capture()?;
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let frame = self.camera.take_one()?;
        Ok(frame.to_vec())
    }
}
