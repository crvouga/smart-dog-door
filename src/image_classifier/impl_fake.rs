use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::logger::interface::Logger;
use rand::distr::{Distribution, Uniform};

pub struct FakeImageClassifier {
    logger: Box<dyn Logger>,
}

impl FakeImageClassifier {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Self {
            logger: logger.with_namespace("image_classifier.fake"),
        }
    }
}

impl ImageClassifier for FakeImageClassifier {
    fn classify(&self, _image: &[u8]) -> Result<Vec<Classification>, Box<dyn std::error::Error>> {
        self.logger
            .info("Classifying image with fake classifier...")?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        let objects = vec![
            "dog", "cat", "person", "car", "chair", "table", "bird", "tree", "bicycle", "book",
            "laptop", "phone", "cup", "bottle", "keyboard", "mouse", "plant", "clock",
        ];

        let mut rng = rand::rng();

        let index_dist = Uniform::new(0, objects.len())?;

        let confidence_dist = Uniform::new(0.0, 1.0)?;

        let classification = Classification {
            label: objects[index_dist.sample(&mut rng)].to_string(),
            confidence: confidence_dist.sample(&mut rng),
        };

        let classifications = vec![classification];

        self.logger.info("Classified image")?;

        Ok(classifications)
    }
}
