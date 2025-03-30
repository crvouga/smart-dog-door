use app::App;
use camera::impl_fake::CameraFake;
use config::Config;
use dog_door::impl_fake::DogDoorFake;
use image_classifier::impl_fake::ImageClassifierFake;
use logger::impl_console::LoggerConsole;

mod app;
mod camera;
mod config;
mod dog_door;
mod image_classifier;
mod logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();

    let logger = Box::new(LoggerConsole::new(config.logger_timezone));

    let camera = Box::new(CameraFake::new(logger.clone()));

    let dog_door = Box::new(DogDoorFake::new(logger.clone()));

    let image_classifier = Box::new(ImageClassifierFake::new(logger.clone()));

    let app = App::new(config, logger, camera, dog_door, image_classifier);

    app.start()?;

    Ok(())
}
