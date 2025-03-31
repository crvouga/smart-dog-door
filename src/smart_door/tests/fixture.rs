use crate::config::Config;
use crate::device_camera::{impl_fake::DeviceCameraFake, interface::DeviceCamera};
use crate::device_display::{impl_console::DeviceDisplayConsole, interface::DeviceDisplay};
use crate::device_door::{impl_fake::DeviceDoorFake, interface::DeviceDoor};
use crate::image_classifier::{impl_fake::ImageClassifierFake, interface::ImageClassifier};
use crate::library::logger::{impl_console::LoggerConsole, interface::Logger};
use crate::smart_door::main::SmartDoor;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct Fixture {
    pub config: Config,
    pub logger: Arc<dyn Logger + Send + Sync>,
    pub device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    pub device_door: Arc<dyn DeviceDoor + Send + Sync>,
    pub device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    pub smart_door: SmartDoor,
}

impl Fixture {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let config = Config::default();
        let logger = Arc::new(LoggerConsole::new(config.logger_timezone));
        let device_camera = Arc::new(DeviceCameraFake::new(logger.clone()));
        let device_door = Arc::new(DeviceDoorFake::new(logger.clone()));
        let device_display = Arc::new(Mutex::new(DeviceDisplayConsole::new()));
        let image_classifier = Arc::new(ImageClassifierFake::new(logger.clone()));
        let smart_door = SmartDoor::new(
            config.clone(),
            logger.clone(),
            device_camera.clone(),
            device_door.clone(),
            device_display.clone(),
            image_classifier.clone(),
        );

        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            smart_door,
        }
    }
}
