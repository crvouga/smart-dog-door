use crate::{
    image_classifier::{impl_fake::ImageClassifierFake, interface::ImageClassifier},
    library::logger::impl_console::LoggerConsole,
};
use std::sync::Arc;

pub struct Fixture {
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

#[cfg(test)]
impl Fixture {
    pub fn new() -> Self {
        let offset = chrono::Local::now().offset().to_owned();
        let logger = Arc::new(LoggerConsole::new(offset));
        let image_classifier = Arc::new(ImageClassifierFake::new(logger));
        Self { image_classifier }
    }
}
