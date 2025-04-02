use super::main::SmartDoor;
use crate::smart_door::core::{Effect, Msg};
use std::time::Instant;

impl SmartDoor {
    pub fn execute_effect(&self, effect: Effect) {
        let _ = self.logger.info(&format!("Running effect: {:?}", effect));

        println!("execute_effect effect: {:?}", effect);

        match effect {
            Effect::SubscribeDoor => {
                let events = self.device_door.events();
                loop {
                    if let Ok(event) = events.recv() {
                        self.send(Msg::DoorEvent(event));
                    }
                }
            }
            Effect::SubscribeCamera => {
                let events = self.device_camera.events();
                loop {
                    if let Ok(event) = events.recv() {
                        self.send(Msg::CameraEvent(event));
                    }
                }
            }
            Effect::SubscribeTick => loop {
                std::thread::sleep(self.config.tick_rate);
                self.send(Msg::Tick(Instant::now()));
            },
            Effect::LockDoor => {
                let locked = self.device_door.lock();
                self.send(Msg::DoorLockDone(locked));
            }
            Effect::UnlockDoor => {
                let unlocked = self.device_door.unlock();
                self.send(Msg::DoorUnlockDone(unlocked));
            }
            Effect::CaptureFrames => {
                let frames = self.device_camera.capture_frame();
                self.send(Msg::FramesCaptureDone(frames));
            }
            Effect::ClassifyFrames { frames } => {
                let classifications = self
                    .image_classifier
                    .classify(frames.iter().map(|f| f.0.clone()).collect());
                self.send(Msg::FramesClassifyDone(classifications));
            }
        }
    }
}
