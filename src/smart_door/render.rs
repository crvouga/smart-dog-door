use crate::config::Config;
use crate::device_display::interface::DeviceDisplay;
use crate::smart_door::core::{Model, ModelDoor};
use std::sync::Arc;
use std::sync::Mutex;

use super::core::{Detection, ModelDeviceConnection};

#[derive(Clone)]
pub struct Render {
    device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    config: Config,
}

impl Render {
    pub fn new(
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        config: Config,
    ) -> Self {
        Self {
            device_display,
            config,
        }
    }

    pub fn render(&self, model: &Model) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut device_display = self.device_display.lock().unwrap();
        device_display.clear()?;

        match model {
            Model::Connecting(connecting) => {
                let camera_text = match connecting.camera {
                    ModelDeviceConnection::Connected => "camera connected",
                    ModelDeviceConnection::Connecting => "camera connecting",
                };
                device_display.write_line(0, camera_text)?;

                let door_text = match connecting.door {
                    ModelDeviceConnection::Connected => "door connected",
                    ModelDeviceConnection::Connecting => "door connecting",
                };
                device_display.write_line(1, door_text)?;
            }
            Model::Ready(ready) => {
                let detection = ready.camera.to_detection(&self.config);
                // Render camera state
                let camera_text = match detection {
                    Detection::Cat => "cat",
                    Detection::Dog => "dog",
                    Detection::None => "none",
                };
                device_display.write_line(0, camera_text)?;

                // Render door state
                let door_text = match ready.door {
                    ModelDoor::Locking { .. } => "locking",
                    ModelDoor::Locked => "locked",
                    ModelDoor::Unlocking { .. } => "unlocking",
                    ModelDoor::Unlocked => "unlocked",
                };
                device_display.write_line(1, door_text)?;
            }
        }

        device_display.render()?;

        Ok(())
    }
}
