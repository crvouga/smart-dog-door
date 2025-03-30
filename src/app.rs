use crate::app_config::AppConfig;
use crate::camera::interface::Camera;
use crate::dog_door::interface::DogDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};

pub struct App {
    camera: Box<dyn Camera>,
    dog_door: Box<dyn DogDoor>,
    image_classifier: Box<dyn ImageClassifier>,
    config: AppConfig,
}

impl App {
    pub fn new(
        config: AppConfig,
        camera: Box<dyn Camera>,
        dog_door: Box<dyn DogDoor>,
        image_classifier: Box<dyn ImageClassifier>,
    ) -> Self {
        Self {
            config,
            camera,
            dog_door,
            image_classifier,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.camera.start()?;

        self.dog_door.lock()?;

        loop {
            let image_frame = self.camera.capture_frame()?;

            let classifications = self.image_classifier.classify(&image_frame)?;

            if self.does_probably_have_dog_in_frame(&classifications)
                && !self.does_probably_have_cat_in_frame(&classifications)
            {
                self.dog_door.unlock()?;
            } else {
                self.dog_door.lock()?;
            }

            std::thread::sleep(self.config.check_rate);
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
}
