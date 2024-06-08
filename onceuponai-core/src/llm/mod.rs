pub mod e5;
pub mod gemma;
pub mod quantized;
pub mod rag;
use anyhow::Result;
use candle_core::Device;

fn parse_device(device_type: Option<String>) -> Result<Device> {
    let device_type = device_type.unwrap_or("cpu".to_string());

    let device = if device_type == "cpu" {
        Device::Cpu
    } else {
        Device::new_cuda(0).unwrap()
    };
    Ok(device)
}

pub struct LLMState {
    pub use_quantized: bool,
}

pub enum LLMModel {
    Gemma,
    QuantizedModel,
}
