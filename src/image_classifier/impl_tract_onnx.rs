use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::image_classifier::tract::image::resize_image_to_tensor;
use image::DynamicImage;
use tract_onnx::prelude::*;

use super::models::model_config::ModelConfig;

pub struct ImageClassifierTractOnnx {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, TypedModel>,
    config: ModelConfig,
}

impl ImageClassifierTractOnnx {
    pub fn new(config: ModelConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let model = tract_onnx::onnx()
            .model_for_path(&config.onnx_model_path)?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self { model, config })
    }
}

impl ImageClassifier for ImageClassifierTractOnnx {
    fn classify(
        &self,
        images: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Vec::new();

        for image in images {
            let input = resize_image_to_tensor(
                &image,
                self.config.input_shape.1, // width
                self.config.input_shape.0, // height
            )?;

            let outputs = self.model.run(tvec!(input.into_tvalue()))?;

            let mut predictions = Vec::new();

            // Handle multiple possible output formats
            for output in outputs {
                if let Ok(output) = output.to_array_view::<f32>() {
                    let shape = output.shape();

                    // Handle different output shapes
                    if shape.len() == 3 {
                        // Standard YOLO output format [batch, num_boxes, num_classes + 5]
                        let num_boxes = shape[1];
                        let num_classes = shape[2] - 5; // 5 for box coords + confidence

                        for i in 0..num_boxes {
                            let confidence = output[[0, i, 4]];
                            if confidence > 0.1 {
                                // Confidence threshold
                                let mut max_class = 0;
                                let mut max_prob = 0.0f32;

                                for j in 0..num_classes {
                                    let prob = output[[0, i, j + 5]];
                                    if prob > max_prob {
                                        max_prob = prob;
                                        max_class = j;
                                    }
                                }

                                predictions.push((max_class, confidence * max_prob));
                            }
                        }
                    }
                }
            }

            predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            predictions.truncate(5); // Keep top 5 predictions

            let classifications = predictions
                .iter()
                .filter_map(|(class_idx, confidence)| {
                    // Adjust these indices based on your model's class mapping
                    match *class_idx {
                        15 => Some(Classification {
                            label: "cat".to_string(),
                            confidence: *confidence,
                        }),
                        16 => Some(Classification {
                            label: "dog".to_string(),
                            confidence: *confidence,
                        }),
                        _ => None,
                    }
                })
                .collect();

            results.push(classifications);
        }

        Ok(results)
    }
}
