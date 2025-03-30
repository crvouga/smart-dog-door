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

        self.camera.start()?;

        self.dog_door.lock()?;

        self.run_dog_door_control_loop()?;

        Ok(())
    }

    fn run_dog_door_control_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let image_frame = self.camera.capture_frame()?;

            let classifications = self.image_classifier.classify(&image_frame)?;

            let is_dog_in_frame = self.does_probably_have_dog_in_frame(&classifications);

            let is_cat_in_frame = self.does_probably_have_cat_in_frame(&classifications);

            let should_unlock = is_dog_in_frame && !is_cat_in_frame;

            self.logger.info(&format!(
                "Dog in frame: {}, Cat in frame: {}, Should unlock: {}",
                is_dog_in_frame, is_cat_in_frame, should_unlock
            ))?;

            if should_unlock {
                self.logger.info("Opening door for dog")?;
                self.dog_door.unlock()?;
            } else if self.dog_door.is_unlocked()? {
                self.logger.info("Closing door")?;
                self.dog_door.lock()?;
            }

            self.sleep()?;
        }
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Stopping app...")?;

        self.camera.stop()?;

        self.dog_door.lock()?;

        self.logger.info("App stopped")?;

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
