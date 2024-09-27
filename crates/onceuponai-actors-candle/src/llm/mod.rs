use anyhow::Result;
use candle_core::Device;
use e5::E5Spec;
use gemma::GemmaSpec;
use mistral::MistralSpec;
use onceuponai_actors::abstractions::{ActorActions, ActorKindActions, ActorMetadata, ActorObject};
use quantized::QuantizedSpec;
use serde::Deserialize;

pub mod e5;
pub mod gemma;
pub mod mistral;
pub mod quantized;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorKind {
    Gemma(ActorObject<GemmaSpec>),
    Quantized(ActorObject<QuantizedSpec>),
    Mistral(ActorObject<MistralSpec>),
    E5(ActorObject<E5Spec>),
}

impl ActorKindActions for ActorKind {
    fn actor(&self) -> Box<dyn ActorActions> {
        match self {
            ActorKind::Gemma(object) => Box::new(object.spec()),
            ActorKind::Quantized(object) => Box::new(object.spec()),
            ActorKind::Mistral(object) => Box::new(object.spec()),
            ActorKind::E5(object) => Box::new(object.spec()),
        }
    }

    fn metadata(&self) -> ActorMetadata {
        match self {
            ActorKind::Gemma(object) => object.metadata(),
            ActorKind::Quantized(object) => object.metadata(),
            ActorKind::Mistral(object) => object.metadata(),
            ActorKind::E5(object) => object.metadata(),
        }
    }
}

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
