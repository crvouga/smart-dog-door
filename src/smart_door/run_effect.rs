use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::ImageClassifier;
use crate::library::logger::interface::Logger;
use crate::smart_door::core::{Effect, Event};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct RunEffect {
    config: Config,
    logger: Arc<dyn Logger + Send + Sync>,
    device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    device_door: Arc<dyn DeviceDoor + Send + Sync>,
    image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    event_sender: Sender<Event>,
}

impl RunEffect {
    pub fn new(
        config: Config,
        logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
        event_sender: Sender<Event>,
    ) -> Self {
        Self {
            config,
            logger,
            device_camera,
            device_door,
            image_classifier,
            event_sender,
        }
    }

    pub fn run_effect(&self, effect: Effect) {
        let _ = self
            .logger
            .info(&format!("Running effect: {:?}", effect.to_display_string()));

        match effect {
            Effect::SubscribeToDoorEvents => {
                let events = self.device_door.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if self.event_sender.send(Event::DoorEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeToCameraEvents => {
                let events = self.device_camera.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if self.event_sender.send(Event::CameraEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeTick => loop {
                std::thread::sleep(self.config.tick_rate);
                if self.event_sender.send(Event::Tick(Instant::now())).is_err() {
                    continue;
                }
            },
            Effect::StartCamera => {
                let started = self.device_camera.start();
                let _ = self.event_sender.send(Event::CameraStartDone(started));
            }
            Effect::LockDoor => {
                let locked = self.device_door.lock();
                let _ = self.event_sender.send(Event::DoorLockDone(locked));
            }
            Effect::UnlockDoor => {
                let unlocked = self.device_door.unlock();
                let _ = self.event_sender.send(Event::DoorUnlockDone(unlocked));
            }
            Effect::CaptureFrames => {
                let frames = self.device_camera.capture_frame();
                let _ = self.event_sender.send(Event::FramesCaptureDone(frames));
            }
            Effect::ClassifyFrames { frames } => {
                let classifications = self.image_classifier.classify(frames.clone());
                let _ = self
                    .event_sender
                    .send(Event::FramesClassifyDone(classifications));
            }
        }
    }
}
