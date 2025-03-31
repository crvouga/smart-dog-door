use super::core::{DoorAction, DoorState};
use crate::config::Config;
use crate::device_display::interface::DeviceDisplay;
use crate::smart_door::core::{CameraState, State};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

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
    pub fn render(&self, state: &State) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut device_display = self.device_display.lock().unwrap();

        device_display.clear()?;

        match state {
            State::Error { message, .. } => {
                device_display.write_line(0, &format!("Error: {}", message))?;
            }
            State::DevicesInitializing { device_states } => {
                match device_states.camera {
                    CameraState::Disconnected => {
                        device_display.write_line(0, "Camera connecting...")?;
                    }
                    CameraState::Connected(time) => {
                        if time.elapsed() > Duration::from_secs(2) {
                            device_display.write_line(0, "Camera connected")?;
                        } else {
                            device_display.write_line(0, "Camera connecting...")?;
                        }
                    }
                    CameraState::Started => {
                        device_display.write_line(0, "Camera connected")?;
                    }
                }

                match device_states.door {
                    DoorState::Disconnected => {
                        device_display.write_line(1, "Door connecting...")?;
                    }
                    DoorState::Connected(time) => {
                        if time.elapsed() > Duration::from_secs(2) {
                            device_display.write_line(1, "Door connected")?;
                        } else {
                            device_display.write_line(1, "Door connecting...")?;
                        }
                    }
                    _ => {
                        device_display.write_line(1, "Door connected")?;
                    }
                }
            }
            State::AnalyzingFramesCapture { .. } | State::AnalyzingFramesClassifying { .. } => {
                device_display.write_line(0, "Analyzing...")?;
            }
            State::ControllingDoor {
                action, start_time, ..
            } => match action {
                DoorAction::Locking => {
                    if start_time.elapsed() > Duration::from_secs(2) {
                        device_display.write_line(0, "Door locked")?;
                    } else {
                        device_display.write_line(0, "Locking door...")?;
                    }
                }
                DoorAction::Unlocking => {
                    if start_time.elapsed() > Duration::from_secs(2) {
                        device_display.write_line(0, "Door unlocked")?;
                    } else {
                        device_display.write_line(0, "Unlocking door...")?;
                    }
                }
            },
            State::UnlockedGracePeriod {
                countdown_start, ..
            } => {
                let remaining = (self.config.unlock_grace_period.as_secs() as i64
                    - countdown_start.elapsed().as_secs() as i64)
                    .max(0);
                device_display.write_line(0, &format!("Door unlocked ({}s)", remaining))?;
            }
            State::LockingGracePeriod {
                countdown_start, ..
            } => {
                let remaining = (self.config.locking_grace_period.as_secs() as i64
                    - countdown_start.elapsed().as_secs() as i64)
                    .max(0);
                device_display.write_line(0, &format!("Locking in {}...", remaining))?;
            }
            State::Idle {
                message,
                message_time,
                ..
            } => {
                if message_time.elapsed() > Duration::from_secs(2) {
                    device_display.write_line(0, "Analyzing...")?;
                } else {
                    // Split message into lines of max 16 chars
                    let mut line = String::new();
                    let mut first = true;
                    for word in message.split_whitespace() {
                        if line.len() + word.len() + 1 <= 16 {
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(word);
                        } else {
                            if first {
                                device_display.write_line(0, &line)?;
                                first = false;
                            } else {
                                device_display.write_line(1, &line)?;
                            }
                            line = word.to_string();
                        }
                    }
                    if first {
                        device_display.write_line(0, &line)?;
                    } else {
                        device_display.write_line(1, &line)?;
                    }
                }
            }
        }

        Ok(())
    }
}
