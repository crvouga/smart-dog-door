use super::core::{Detection, ModelDeviceConnection};
use super::main::SmartDoor;
use crate::smart_door::core::to_detection;
use crate::smart_door::core::{Model, ModelDoor};
use std::sync::Arc;

impl SmartDoor {
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
                let detection = to_detection(&ready.camera, &self.config);
                // Render camera state
                let camera_text = match detection {
                    Detection::Cat => "cat",
                    Detection::Dog => "dog",
                    Detection::None => "none",
                };
                device_display.write_line(0, camera_text)?;

                // Render door state
                let door_text = match ready.door {
                    ModelDoor::WillClose { .. } => "locking...",
                    ModelDoor::Closed => "locked",
                    ModelDoor::WillOpen { .. } => "unlocking...",
                    ModelDoor::Opened => "unlocked",
                };
                device_display.write_line(1, door_text)?;
            }
        }

        device_display.render()?;

        Ok(())
    }
}
