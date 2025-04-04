use crate::image_classifier::models::model_config::ModelConfig;
use crate::image_classifier::{
    impl_tract_onnx::ImageClassifierTractOnnx, interface::ImageClassifier,
};
use std::sync::Arc;

pub struct Fixture {
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

#[cfg(test)]
impl Fixture {
    pub fn new() -> Self {
        let configs = vec![
            ModelConfig {
                onnx_model_path: "./src/image_classifier/models/mobilenetv2-7.onnx".to_string(),
                input_shape: (224, 224),
            },
            ModelConfig {
                onnx_model_path: "./src/image_classifier/models/yolov5s.onnx".to_string(),
                input_shape: (640, 640),
            },
        ];

        let config = configs[1].clone();

        let image_classifier = Arc::new(ImageClassifierTractOnnx::new(config).unwrap());

        Self { image_classifier }
    }
}
