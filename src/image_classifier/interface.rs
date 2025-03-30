#[derive(Debug)]
pub struct Classification {
    pub label: String,
    pub confidence: f32,
}

pub trait ImageClassifier {
    fn classify(&self, image: &[u8]) -> Result<Vec<Classification>, Box<dyn std::error::Error>>;
}
