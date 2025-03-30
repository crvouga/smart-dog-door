mod app;
mod app_config;
mod camera;
mod dog_door;
mod image_classifier;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = app_config::AppConfig {
        classification_min_confidence_dog: 0.7,
        classification_min_confidence_cat: 0.7,
        check_rate: std::time::Duration::from_secs(10),
    };

    let camera = camera::impl_fake::FakeCamera::new();
    let dog_door = dog_door::impl_fake::FakeDogDoor::new();
    let image_classifier = image_classifier::impl_fake::FakeImageClassifier::new();

    let app = app::App::new(
        config,
        Box::new(camera),
        Box::new(dog_door),
        Box::new(image_classifier),
    );

    app.start()?;

    Ok(())
}
