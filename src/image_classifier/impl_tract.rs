use crate::image_classifier::interface::{Classification, ImageClassifier};
use image::DynamicImage;
use std::collections::HashMap;
use tract_onnx::prelude::*;

pub struct ImageClassifierTract {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, TypedModel>,
    last_process_time: std::time::Instant,
    min_process_interval: std::time::Duration,
    class_mapping: HashMap<usize, String>,
}

impl ImageClassifierTract {
    pub fn new(
        model_path: &str,
        process_interval: std::time::Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Load the model
        let model = tract_onnx::onnx()
            .model_for_path(model_path)?
            .into_optimized()?
            .into_runnable()?;

        // Initialize class mapping for ImageNet classes
        let mut class_mapping = HashMap::new();
        // Cat classes in ImageNet
        class_mapping.insert(281, "cat".to_string()); // tabby cat
        class_mapping.insert(282, "cat".to_string()); // tiger cat
        class_mapping.insert(283, "cat".to_string()); // Persian cat
        class_mapping.insert(284, "cat".to_string()); // Siamese cat
        class_mapping.insert(285, "cat".to_string()); // Egyptian cat
                                                      // Dog classes in ImageNet
        class_mapping.insert(151, "dog".to_string()); // Chihuahua
        class_mapping.insert(152, "dog".to_string()); // Japanese spaniel
        class_mapping.insert(153, "dog".to_string()); // Maltese dog
        class_mapping.insert(154, "dog".to_string()); // Pekinese
        class_mapping.insert(155, "dog".to_string()); // Shih-Tzu

        Ok(Self {
            model,
            last_process_time: std::time::Instant::now(),
            min_process_interval: process_interval,
            class_mapping,
        })
    }

    fn preprocess_image(
        &self,
        image: &DynamicImage,
    ) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
        // Resize to MobileNetV2's expected input size (224x224)
        let resized = image.resize_exact(224, 224, image::imageops::FilterType::Triangle);

        // Convert to RGB format
        let rgb = resized.to_rgb8();

        // Convert to tensor and normalize to [-1, 1]
        let tensor = tract_ndarray::Array4::from_shape_fn((1, 3, 224, 224), |(_, c, y, x)| {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            (pixel[c] as f32 / 127.5) - 1.0
        });

        // Convert to Tensor
        let tensor = tensor.into_tensor();
        Ok(tensor)
    }
}

impl ImageClassifier for ImageClassifierTract {
    fn classify(
        &self,
        images: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if enough time has passed since last processing
        let now = std::time::Instant::now();
        if now.duration_since(self.last_process_time) < self.min_process_interval {
            return Ok(vec![]); // Return empty if called too soon
        }

        // Process all images
        let mut results = Vec::new();
        for image in images {
            // Preprocess image
            let input = self.preprocess_image(&image)?;

            // Run inference
            let outputs = self.model.run(tvec!(input.into_tvalue()))?;
            let output = outputs[0].to_array_view::<f32>()?;

            // Process results - find top 5 predictions
            let mut predictions: Vec<(usize, f32)> = output
                .iter()
                .enumerate()
                .map(|(i, &score)| (i, score))
                .collect();
            predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            predictions.truncate(5);

            // Convert to our classification format
            let mut classifications = Vec::new();
            for (class_idx, confidence) in predictions {
                if let Some(label) = self.class_mapping.get(&class_idx) {
                    classifications.push(Classification {
                        label: label.clone(),
                        confidence,
                    });
                }
            }
            results.push(classifications);
        }

        Ok(results)
    }
}
