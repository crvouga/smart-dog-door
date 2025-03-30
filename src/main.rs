mod app;
mod app_config;
mod camera;
mod dog_door;
mod image_classifier;
mod logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = app_config::AppConfig {
        classification_rate: std::time::Duration::from_secs(3),
        classification_min_confidence_dog: 0.7,
        classification_min_confidence_cat: 0.7,
    };

    let timezone = mountain_standard_time();

    let logger = logger::impl_console::ConsoleLogger::new(timezone);

    let camera = camera::impl_fake::FakeCamera::new(Box::new(logger.clone()));

    let dog_door = dog_door::impl_fake::FakeDogDoor::new(Box::new(logger.clone()));

    let image_classifier =
        image_classifier::impl_fake::FakeImageClassifier::new(Box::new(logger.clone()));

    let app = app::App::new(
        config,
        Box::new(logger.clone()),
        Box::new(camera),
        Box::new(dog_door),
        Box::new(image_classifier),
    );

    app.start()?;

    Ok(())
}

fn mountain_standard_time() -> chrono::FixedOffset {
    chrono::FixedOffset::west_opt(7 * 3600).unwrap()
}
