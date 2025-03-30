use crate::camera::interface::Camera;
use crate::config::Config;
use crate::dog_door::interface::DogDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::logger::interface::Logger;

pub struct App {
    config: Config,
    logger: Box<dyn Logger>,
    camera: Box<dyn Camera>,
    dog_door: Box<dyn DogDoor>,
    image_classifier: Box<dyn ImageClassifier>,
}

impl App {
    pub fn new(
        config: Config,
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

        self.main_loop(self.logger.with_namespace("main_loop"))?;

        self.logger.info("Stopping camera...")?;

        self.camera.stop()?;

        self.logger.info("Camera stopped")?;

        Ok(())
    }

    fn main_loop(&self, logger: Box<dyn Logger>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            logger.info("Capturing image...")?;

            let image_frame = self.camera.capture_frame()?;

            logger.info("Image captured")?;

            logger.info("Classifying image...")?;

            let classifications = self.image_classifier.classify(&image_frame)?;

            logger.info("Image classified")?;

            logger.info("Checking if dog is in frame...")?;

            let is_dog_in_frame = self.does_probably_have_dog_in_frame(&classifications);

            logger.info(&format!("Dog in frame: {}", is_dog_in_frame))?;

            let is_cat_in_frame = self.does_probably_have_cat_in_frame(&classifications);

            logger.info(&format!("Cat in frame: {}", is_cat_in_frame))?;

            let should_unlock = is_dog_in_frame && !is_cat_in_frame;

            if should_unlock {
                logger.info("Dog is in frame and cat is not, unlocking dog door...")?;

                self.dog_door.unlock()?;

                logger.info("Dog door unlocked")?;
            } else {
                logger.info("Dog is not in frame, locking dog door...")?;

                self.dog_door.lock()?;

                logger.info("Dog door locked")?;
            }

            logger.info(&format!(
                "Going to sleep for {} seconds...",
                self.config.classification_rate.as_secs()
            ))?;

            self.sleep(logger.clone())?;

            logger.info("Waking up...")?;
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

    fn sleep(&self, logger: Box<dyn Logger>) -> Result<(), Box<dyn std::error::Error>> {
        let sleep_duration = self.config.classification_rate;
        let start_time = std::time::Instant::now();
        while start_time.elapsed() < sleep_duration {
            logger.info("Sleeping...")?;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }
}
