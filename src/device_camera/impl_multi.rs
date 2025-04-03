use image::DynamicImage;

use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
use std::sync::{mpsc, Arc};

pub struct MultiDeviceCamera {
    cameras: Vec<Arc<dyn DeviceCamera + Send + Sync>>,
}

impl MultiDeviceCamera {
    pub fn new(cameras: Vec<Arc<dyn DeviceCamera + Send + Sync>>) -> Self {
        Self { cameras }
    }
}

impl DeviceCamera for MultiDeviceCamera {
    fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for camera in &self.cameras {
            camera.start()?;
        }
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for camera in &self.cameras {
            camera.stop()?;
        }
        Ok(())
    }

    fn capture_frame(&self) -> Result<Vec<DynamicImage>, Box<dyn std::error::Error + Send + Sync>> {
        let mut frames = Vec::new();
        for camera in &self.cameras {
            frames.extend(camera.capture_frame()?);
        }
        Ok(frames)
    }

    fn events(&self) -> mpsc::Receiver<DeviceCameraEvent> {
        let (tx, rx) = mpsc::channel();
        let cameras = self.cameras.clone();

        std::thread::spawn(move || {
            let receivers: Vec<_> = cameras.iter().map(|camera| camera.events()).collect();

            let mut connected_count = 0;

            loop {
                for receiver in &receivers {
                    if let Ok(event) = receiver.try_recv() {
                        match event {
                            DeviceCameraEvent::Connected => {
                                connected_count += 1;
                                if connected_count == cameras.len() {
                                    let _ = tx.send(DeviceCameraEvent::Connected);
                                }
                            }
                            DeviceCameraEvent::Disconnected => {
                                connected_count = connected_count.saturating_sub(1);
                                if connected_count == 0 {
                                    let _ = tx.send(DeviceCameraEvent::Disconnected);
                                }
                            }
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        rx
    }
}
