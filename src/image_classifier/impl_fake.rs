use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use rand::distr::{Distribution, Uniform};
use std::sync::Arc;
pub struct ImageClassifierFake {
    logger: Arc<dyn Logger + Send + Sync>,
}

impl ImageClassifierFake {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self {
            logger: logger
                .with_namespace("image_classifier")
                .with_namespace("fake"),
        }
    }
}

impl ImageClassifier for ImageClassifierFake {
    fn classify(
        &self,
        _image: &[u8],
    ) -> Result<Vec<Classification>, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Classifying image...")?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        // let objects = vec![
        //     "dog", "cat", "person", "car", "chair", "table", "bird", "tree", "bicycle", "book",
        //     "laptop", "phone", "cup", "bottle", "keyboard", "mouse", "plant", "clock",
        // ];

        let objects = vec!["dog"];

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
