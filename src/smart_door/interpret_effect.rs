use super::main::SmartDoor;
use crate::smart_door::core::{Effect, Msg};
use std::time::Instant;

impl SmartDoor {
    pub fn interpret_effect(&self, effect: Effect) {
        let _ = self.logger.info(&format!("Running effect: {:?}", effect));

        match effect {
            Effect::SubscribeDoor => {
                let events = self.device_door.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if self.event_sender.send(Msg::DoorEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeCamera => {
                let events = self.device_camera.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if self.event_sender.send(Msg::CameraEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeTick => loop {
                std::thread::sleep(self.config.tick_rate);
                if self.event_sender.send(Msg::Tick(Instant::now())).is_err() {
                    continue;
                }
            },
            Effect::LockDoor => {
                let locked = self.device_door.lock();
                let _ = self.event_sender.send(Msg::DoorLockDone(locked));
            }
            Effect::UnlockDoor => {
                let unlocked = self.device_door.unlock();
                let _ = self.event_sender.send(Msg::DoorUnlockDone(unlocked));
            }
            Effect::CaptureFrames => {
                let frames = self.device_camera.capture_frame();
                let _ = self.event_sender.send(Msg::FramesCaptureDone(frames));
            }
            Effect::ClassifyFrames { frames } => {
                let classifications = self
                    .image_classifier
                    .classify(frames.iter().map(|f| f.0.clone()).collect());

                let _ = self
                    .event_sender
                    .send(Msg::FramesClassifyDone(classifications));
            }
        }
    }
}
