use app::App;
use config::Config;
use device_camera::impl_fake::DeviceCameraFake;
use device_dog_door::impl_fake::DeviceDogDoorFake;
use image_classifier::impl_fake::ImageClassifierFake;
use logger::impl_console::LoggerConsole;

mod app;
mod app;
mod config;
mod device_camera;
mod device_display;
mod device_dog_door;
mod image_classifier;
mod logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();

    let logger = Box::new(LoggerConsole::new(config.logger_timezone));

    let camera = Box::new(DeviceCameraFake::new(logger.clone()));

    let dog_door = Box::new(DeviceDogDoorFake::new(logger.clone()));

    let image_classifier = Box::new(ImageClassifierFake::new(logger.clone()));

    let app = App::new(config, logger, camera, dog_door, image_classifier);

    app.start()?;

    Ok(())
}
