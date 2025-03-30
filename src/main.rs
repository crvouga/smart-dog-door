use std::sync::Arc;

use crate::{
    config::Config, device_camera::impl_fake::DeviceCameraFake,
    device_display::impl_fake::DeviceDisplayFake, device_dog_door::impl_fake::DeviceDogDoorFake,
    image_classifier::impl_fake::ImageClassifierFake, library::logger::impl_console::LoggerConsole,
    smart_dog_door::SmartDogDoor,
};

mod config;
mod device_camera;
mod device_display;
mod device_dog_door;
mod image_classifier;
mod library;
mod smart_dog_door;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();

    let logger = Arc::new(LoggerConsole::new(config.logger_timezone));

    let device_camera = Arc::new(DeviceCameraFake::new(logger.clone()));

    let device_dog_door = Arc::new(DeviceDogDoorFake::new(logger.clone()));

    let device_display = Arc::new(DeviceDisplayFake::new(logger.clone()));

    let image_classifier = Arc::new(ImageClassifierFake::new(logger.clone()));

    let smart_dog_door = SmartDogDoor::new(
        config,
        logger,
        device_camera,
        device_dog_door,
        device_display,
        image_classifier,
    );

    smart_dog_door.run()?;

    Ok(())
}
