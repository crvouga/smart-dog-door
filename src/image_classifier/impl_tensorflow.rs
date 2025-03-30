use crate::image_classifier::interface::{Classification, ImageClassifier};
use image::{DynamicImage, ImageBuffer};
use tflite::{FlatBufferModel, InterpreterBuilder};

pub struct TensorFlowClassifier {
    interpreter: tflite::Interpreter,
    logger: Box<dyn crate::logger::interface::Logger>,
    last_process_time: std::time::Instant,
    min_process_interval: std::time::Duration,
}

impl TensorFlowClassifier {
    pub fn new(
        model_path: &str,
        logger: Box<dyn crate::logger::interface::Logger>,
        process_interval: std::time::Duration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = FlatBufferModel::build_from_file(model_path)?;
        let mut interpreter = InterpreterBuilder::new(model, logger)?.build()?;

        // Configure interpreter for optimal performance
        interpreter.allocate_tensors()?;

        Ok(Self {
            interpreter,
            logger: logger.with_namespace("tensorflow_classifier"),
            last_process_time: std::time::Instant::now(),
            min_process_interval: process_interval,
        })
    }

    fn preprocess_image(&self, image_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let img = image::load_from_memory(image_data)?;

        // Resize to model's expected input size (typically 224x224 or 300x300)
        let resized = img.resize_exact(224, 224, image::imageops::FilterType::Triangle);

        // Convert to RGB format if needed
        let rgb = resized.to_rgb8();

        // Normalize pixel values to 0-1 range
        let normalized: Vec<u8> = rgb.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();

        Ok(normalized)
    }
}

impl ImageClassifier for TensorFlowClassifier {
    fn classify(&self, image: &[u8]) -> Result<Vec<Classification>, Box<dyn std::error::Error>> {
        // Check if enough time has passed since last processing
        let now = std::time::Instant::now();
        if now.duration_since(self.last_process_time) < self.min_process_interval {
            return Ok(vec![]); // Return empty if called too soon
        }

        self.logger.debug("Starting image classification")?;

        // Preprocess image
        let processed_data = self.preprocess_image(image)?;

        // Set input tensor
        let input_tensor = self.interpreter.input_tensor(0)?;
        input_tensor.copy_from_buffer(&processed_data)?;

        // Run inference
        self.interpreter.invoke()?;

        // Get output
        let output_tensor = self.interpreter.output_tensor(0)?;
        let output_data: Vec<f32> = output_tensor.data().copy()?;

        // Process results - assuming output is [cat_confidence, dog_confidence]
        let classifications = vec![
            Classification {
                label: "cat".to_string(),
                confidence: output_data[0],
            },
            Classification {
                label: "dog".to_string(),
                confidence: output_data[1],
            },
        ];

        self.logger
            .debug(&format!("Classifications: {:?}", classifications))?;

        Ok(classifications)
    }
}
