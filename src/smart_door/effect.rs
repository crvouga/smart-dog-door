use super::main::SmartDoor;
use crate::smart_door::core::{Effect, Msg};
use std::time::Instant;

impl SmartDoor {
    pub fn execute_effect(&self, effect: Effect) {
        let _ = self.logger.info(&format!("Running effect: {:?}", effect));

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
            Effect::OpenDoor => {
                let locked = self.device_door.open();
                self.send(Msg::DoorCloseDone(locked));
            }
            Effect::CloseDoor => {
                let unlocked = self.device_door.close();
                self.send(Msg::DoorOpenDone(unlocked));
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
