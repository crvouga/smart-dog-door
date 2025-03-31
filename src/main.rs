use device_camera::impl_multi::MultiDeviceCamera;
use device_display::impl_gui::DeviceDisplayGui;

use crate::{
    config::Config, device_camera::impl_fake::DeviceCameraFake,
    device_door::impl_fake::DeviceDoorFake, image_classifier::impl_fake::ImageClassifierFake,
    library::logger::impl_console::LoggerConsole, smart_door::SmartDoor,
};
use std::sync::{Arc, Mutex};

mod config;
mod device_camera;
mod device_display;
mod device_door;
mod image_classifier;
mod library;
mod smart_door;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();

    let logger = Arc::new(LoggerConsole::new(config.logger_timezone));

    let device_camera = Arc::new(MultiDeviceCamera::new(vec![
        Arc::new(DeviceCameraFake::new(logger.clone())),
        Arc::new(DeviceCameraFake::new(logger.clone())),
    ]));

    let device_door = Arc::new(DeviceDoorFake::new(logger.clone()));

    let device_display = Arc::new(Mutex::new(DeviceDisplayGui::new()));

    let image_classifier = Arc::new(ImageClassifierFake::new(logger.clone()));

    let smart_door = SmartDoor::new(
        config,
        logger,
        device_camera,
        device_door,
        device_display,
        image_classifier,
    );

    smart_door.run()?;

    Ok(())
}
