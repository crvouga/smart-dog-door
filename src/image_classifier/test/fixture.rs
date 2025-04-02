use super::super::impl_fake::ImageClassifierFake;
use super::super::interface::ImageClassifier;
use std::sync::Arc;

#[cfg(test)]
pub struct Fixture {
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            image_classifier: Arc::new(ImageClassifierFake::new()),
        }
    }
}
