pub mod e5;
pub mod gemma;
pub mod quantized;
pub mod rag;
use anyhow::Result;
use candle_core::Device;

fn parse_device(device_type: &str) -> Result<Device> {
    let device = if device_type == "cpu" {
        Device::Cpu
    } else {
        Device::new_cuda(0).unwrap()
    };
    Ok(device)
}

pub struct LLMState {
    pub eos_token: u32,
}

pub enum LLMModel {
    Gemma,
    QuantizedModel,
}
