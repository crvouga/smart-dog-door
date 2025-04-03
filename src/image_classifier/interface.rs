use image::DynamicImage;

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub confidence: f32,
    pub label: String,
}
pub trait ImageClassifier {
    fn classify(
        &self,
        frames: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>;
}
