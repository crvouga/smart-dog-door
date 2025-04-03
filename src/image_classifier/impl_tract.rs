use crate::image_classifier::interface::{Classification, ImageClassifier};
use image::DynamicImage;
use std::collections::HashMap;
use tract_onnx::prelude::*;

pub struct ImageClassifierTract {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, TypedModel>,
    class_mapping: HashMap<usize, String>,
}

impl ImageClassifierTract {
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Load the model
        let model = tract_onnx::onnx()
            .model_for_path(model_path)?
            .into_optimized()?
            .into_runnable()?;

        // Initialize class mapping (can be COCO or ImageNet depending on model)
        let mut class_mapping = HashMap::new();
        // Common pet classes
        class_mapping.insert(15, "cat".to_string()); // COCO cat
        class_mapping.insert(16, "dog".to_string()); // COCO dog
        class_mapping.insert(281, "cat".to_string()); // ImageNet tabby cat
        class_mapping.insert(282, "cat".to_string()); // ImageNet tiger cat
        class_mapping.insert(283, "cat".to_string()); // ImageNet Persian cat
        class_mapping.insert(151, "dog".to_string()); // ImageNet Chihuahua
        class_mapping.insert(152, "dog".to_string()); // ImageNet Japanese spaniel
        class_mapping.insert(153, "dog".to_string()); // ImageNet Maltese dog

        Ok(Self {
            model,
            class_mapping,
        })
    }

    fn preprocess_image(
        &self,
        image: &DynamicImage,
    ) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
        // Use a simpler MobileNet preprocessing approach
        // Resize to 224x224 (standard for most models)
        let resized = image.resize_exact(224, 224, image::imageops::FilterType::Triangle);

        // Convert to RGB format
        let rgb = resized.to_rgb8();

        // Convert to tensor with NCHW format for most models
        let tensor = tract_ndarray::Array4::from_shape_fn((1, 3, 224, 224), |(_, c, y, x)| {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            pixel[c] as f32 / 255.0 // Simple [0,1] normalization
        });

        Ok(tensor.into_tensor())
    }
}

impl ImageClassifier for ImageClassifierTract {
    fn classify(
        &self,
        images: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Vec::new();

        for image in images {
            // Preprocess image and run model
            let input = self.preprocess_image(&image)?;
            let outputs = self.model.run(tvec!(input.into_tvalue()))?;
            let output = outputs[0].to_array_view::<f32>()?;

            // Simple classification approach
            let mut predictions = Vec::new();

            // Handle common output formats
            match output.shape() {
                // MobileNet style output (1000 classes)
                &[1, 1000] | &[1, 1001] => {
                    for (idx, &score) in output.iter().enumerate().take(1000) {
                        if score > 0.1 {
                            // Only consider reasonable confidence
                            predictions.push((idx, score));
                        }
                    }
                }

                // Other formats - handle more generically
                _ => {
                    // Simple max-finding approach
                    let shape = output.shape();
                    if shape.len() >= 2 {
                        for i in 0..output.shape()[1].min(1000) {
                            if let Some(&score) = output.get([0, i]) {
                                if score > 0.1 {
                                    predictions.push((i, score));
                                }
                            }
                        }
                    }
                }
            }

            // Sort by confidence and take top 5
            predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            predictions.truncate(5);

            // Convert to classifications
            let classifications = predictions
                .iter()
                .filter_map(|(class_idx, confidence)| {
                    self.class_mapping
                        .get(class_idx)
                        .map(|label| Classification {
                            label: label.clone(),
                            confidence: *confidence,
                        })
                })
                .collect();

            results.push(classifications);
        }

        Ok(results)
    }
}
