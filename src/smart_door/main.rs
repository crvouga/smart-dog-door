use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::ImageClassifier;
use crate::library::logger::interface::Logger;
use crate::smart_door::core::Msg;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SmartDoor {
    pub chan: Arc<Mutex<(Sender<Msg>, Receiver<Msg>)>>,
    pub config: Config,
    pub logger: Arc<dyn Logger + Send + Sync>,
    pub device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    pub device_door: Arc<dyn DeviceDoor + Send + Sync>,
    pub device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

impl SmartDoor {
    pub fn new(
        config: Config,
        logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            chan: Arc::new(Mutex::new(channel())),
        }
    }

    pub fn send(&self, msg: Msg) {
        let _ = self.chan.lock().unwrap().0.send(msg);
    }

    pub fn recv(&self) -> Msg {
        self.chan.lock().unwrap().1.recv().unwrap()
    }
}
