use crate::image_classifier::interface::{Classification, ImageClassifier};
use rand::distr::{Distribution, Uniform};

pub struct FakeImageClassifier {}

impl FakeImageClassifier {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImageClassifier for FakeImageClassifier {
    fn classify(&self, _image: &[u8]) -> Result<Vec<Classification>, Box<dyn std::error::Error>> {
        println!("Classifying image with fake classifier...");
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

        Ok(classifications)
    }
}
