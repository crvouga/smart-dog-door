use app::App;
use camera::impl_fake::CameraFake;
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
    let config = config::Config {
        classification_rate: std::time::Duration::from_secs(3),
        classification_min_confidence_dog: 0.7,
        classification_min_confidence_cat: 0.7,
        logger_timezone: mountain_standard_time(),
    };

    let logger = Box::new(LoggerConsole::new(config.logger_timezone));

    let camera = Box::new(CameraFake::new(logger.clone()));

    let dog_door = Box::new(DogDoorFake::new(logger.clone()));

    let image_classifier = Box::new(ImageClassifierFake::new(logger.clone()));

    let app = App::new(config, logger, camera, dog_door, image_classifier);

    app.start()?;

    Ok(())
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
