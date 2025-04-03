use image::DynamicImage;

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub confidence: f32, // probability between 0.0 and 1.0
    pub label: String,
}
pub trait ImageClassifier {
    fn classify(
        &self,
        frames: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>;
}
