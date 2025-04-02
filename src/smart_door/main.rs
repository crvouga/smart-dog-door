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
    pub sender: Sender<Msg>,
    pub receiver: Arc<Mutex<Receiver<Msg>>>,
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
        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));
        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            sender,
            receiver,
        }
    }

    pub fn send(&self, msg: Msg) {
        println!("send msg: {:?}", msg);
        let _ = self.sender.send(msg);
    }

    pub fn recv(&self) -> Msg {
        self.receiver.lock().unwrap().recv().unwrap()
    }
}
