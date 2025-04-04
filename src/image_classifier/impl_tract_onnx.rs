use super::models::model_config::ModelConfig;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use image::DynamicImage;
use tract_onnx::prelude::*;

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

fn load_labels() -> Vec<String> {
    include_str!("imagenet_labels.txt")
        .lines()
        .map(|s| s.to_string())
        .collect()
}

impl ImageClassifier for ImageClassifierTractOnnx {
    fn classify(
        &self,
        images: Vec<DynamicImage>,
    ) -> Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Vec::new();

        for image in images {
            let width = self.config.input_shape.0;
            let height = self.config.input_shape.1;
            let resized = image.resize_exact(width, height, image::imageops::FilterType::Triangle);
            let rgb = resized.to_rgb8();

            let input_tensor: Tensor = tract_ndarray::Array4::from_shape_fn(
                (1, 3, height as usize, width as usize),
                |(_, c, y, x)| rgb[(x as u32, y as u32)][c] as f32 / 255.0,
            )
            .into();

            let output = self.model.run(tvec!(input_tensor.into()))?;
            let output_tensor = output[0].to_array_view::<f32>()?;

            let labels = load_labels(); // Ensure this has correct class mappings
            let mut classifications = Vec::new();
            for (i, &confidence) in output_tensor.iter().enumerate() {
                if confidence > 0.1 && i < labels.len() {
                    classifications.push(Classification {
                        label: labels[i].clone(),
                        confidence,
                    });
                }
            }

            // High confidence first
            classifications.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

            results.push(classifications);
        }

        Ok(results)
    }
}
