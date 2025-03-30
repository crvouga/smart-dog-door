use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
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

pub struct DeviceCameraRaspberryPi {
    camera: SimpleCamera,
    config: RaspberryPiCameraConfig,
    logger: Box<dyn crate::logger::interface::Logger>,
    event_thread: Option<std::thread::JoinHandle<()>>,
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl DeviceCameraRaspberryPi {
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
            event_thread: None,
            shutdown_tx: None,
        })
    }
}

impl DeviceCamera for DeviceCameraRaspberryPi {
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

    fn events(&self) -> std::sync::mpsc::Sender<DeviceCameraEvent> {
        let (event_tx, _) = std::sync::mpsc::channel();
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
        let camera = self.camera.clone();

        let handle = std::thread::spawn(move || {
            let mut was_connected = false;
            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                let is_connected = camera.take_one().is_ok();

                if is_connected && !was_connected {
                    if event_tx.send(DeviceCameraEvent::Connected).is_err() {
                        break; // Exit if receiver is dropped
                    }
                } else if !is_connected && was_connected {
                    if event_tx.send(DeviceCameraEvent::Disconnected).is_err() {
                        break; // Exit if receiver is dropped
                    }
                }

                was_connected = is_connected;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        self.event_thread = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);

        event_tx
    }
}

impl Drop for DeviceCameraRaspberryPi {
    fn drop(&mut self) {
        // Signal thread to shut down
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for thread to finish
        if let Some(handle) = self.event_thread.take() {
            let _ = handle.join();
        }

        // Stop camera capture
        if let Err(e) = self.stop() {
            eprintln!("Failed to stop camera during shutdown: {}", e);
        }
    }
}
