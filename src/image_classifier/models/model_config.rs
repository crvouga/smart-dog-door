#[derive(Debug, Clone, PartialEq)]
pub struct ModelConfig {
    pub onnx_model_path: String,
    pub input_shape: (u32, u32),
}
