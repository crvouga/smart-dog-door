use crate::app_config::AppConfig;
use crate::camera::interface::Camera;
use crate::dog_door::interface::DogDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::logger::interface::Logger;

pub struct App {
    camera: Box<dyn Camera>,
    dog_door: Box<dyn DogDoor>,
    image_classifier: Box<dyn ImageClassifier>,
    config: AppConfig,
    logger: Box<dyn Logger>,
}

impl App {
    pub fn new(
        config: AppConfig,
        logger: Box<dyn Logger>,
        camera: Box<dyn Camera>,
        dog_door: Box<dyn DogDoor>,
        image_classifier: Box<dyn ImageClassifier>,
    ) -> Self {
        Self {
            config,
            camera,
            dog_door,
            image_classifier,
            logger: logger.with_namespace("app"),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Starting app...")?;

        self.logger.info("Starting camera...")?;

        self.camera.start()?;

        self.logger.info("Camera started")?;

        self.logger.info("Locking dog door...")?;

        self.dog_door.lock()?;

        self.logger.info("Dog door locked")?;

        loop {
            self.logger.info("Capturing image...")?;

            let image_frame = self.camera.capture_frame()?;

            self.logger.info("Image captured")?;

            self.logger.info("Classifying image...")?;

            let classifications = self.image_classifier.classify(&image_frame)?;

            self.logger.info("Image classified")?;

            self.logger.info("Checking if dog is in frame...")?;

            let is_dog_in_frame = self.does_probably_have_dog_in_frame(&classifications);

            self.logger
                .info(&format!("Dog in frame: {}", is_dog_in_frame))?;

            let is_cat_in_frame = self.does_probably_have_cat_in_frame(&classifications);

            self.logger
                .info(&format!("Cat in frame: {}", is_cat_in_frame))?;

            let should_unlock = is_dog_in_frame && !is_cat_in_frame;

            if should_unlock {
                self.logger
                    .info("Dog is in frame and cat is not, unlocking dog door...")?;

                self.dog_door.unlock()?;

                self.logger.info("Dog door unlocked")?;
            } else {
                self.logger
                    .info("Dog is not in frame, locking dog door...")?;

                self.dog_door.lock()?;

                self.logger.info("Dog door locked")?;
            }

            self.logger.info(&format!(
                "Going to sleep for {} seconds...",
                self.config.classification_rate.as_secs()
            ))?;

            self.sleep()?;

            self.logger.info("Waking up...")?;
        }
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.camera.stop()?;
        println!("Stopping app...");
        Ok(())
    }

    fn does_probably_have_dog_in_frame(&self, classifications: &[Classification]) -> bool {
        classifications.iter().any(|c| {
            c.label.to_lowercase().contains("dog")
                && c.confidence >= self.config.classification_min_confidence_dog
        })
    }

    fn does_probably_have_cat_in_frame(&self, classifications: &[Classification]) -> bool {
        classifications.iter().any(|c| {
            c.label.to_lowercase().contains("cat")
                && c.confidence >= self.config.classification_min_confidence_cat
        })
    }

    fn sleep(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sleep_duration = self.config.classification_rate;
        let start_time = std::time::Instant::now();
        while start_time.elapsed() < sleep_duration {
            self.logger.info("Sleeping...")?;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }
}
