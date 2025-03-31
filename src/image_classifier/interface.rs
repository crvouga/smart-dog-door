#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub label: String,
    pub confidence: f32,
}

pub trait ImageClassifier {
    fn classify(
        &self,
        frames: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>;
}
