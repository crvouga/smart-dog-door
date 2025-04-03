use crate::image_classifier::{impl_tract::ImageClassifierTract, interface::ImageClassifier};
use std::sync::Arc;

pub struct Fixture {
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

#[cfg(test)]
impl Fixture {
    pub fn new() -> Self {
        let path_mobilenet = "./src/image_classifier/models/mobilenetv2-7.onnx";
        let path_yolov5 = "./src/image_classifier/models/yolov5s.onnx";

        let image_classifier = Arc::new(ImageClassifierTract::new(path_yolov5).unwrap());
        Self { image_classifier }
    }
}
