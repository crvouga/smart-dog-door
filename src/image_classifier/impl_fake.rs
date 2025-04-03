use image::DynamicImage;

use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct ImageClassifierFake {
    logger: Arc<dyn Logger + Send + Sync>,
    counter: AtomicUsize,
}

impl ImageClassifierFake {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self {
            logger: logger
                .with_namespace("image_classifier")
                .with_namespace("fake"),
            counter: AtomicUsize::new(0),
        }
    }
}

impl ImageClassifier for ImageClassifierFake {
    fn classify(
        &self,
        _images: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Classifying image...")?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        let count = self.counter.fetch_add(1, Ordering::SeqCst);

        let classification = match count % 4 {
            0 => Classification {
                label: "dog".to_string(),
                confidence: 0.9,
            },
            1 => Classification {
                label: "cat".to_string(),
                confidence: 0.8,
            },
            2 => Classification {
                label: "dog".to_string(),
                confidence: 0.3,
            },
            3 => Classification {
                label: "cat".to_string(),
                confidence: 0.2,
            },
            _ => unreachable!(),
        };

        let classifications = vec![classification];

        self.logger.info("Classified image")?;

        Ok(vec![classifications])
    }
}
