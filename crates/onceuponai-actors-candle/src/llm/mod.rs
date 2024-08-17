use e5::E5Spec;
use gemma::GemmaSpec;
use onceuponai_actors::abstractions::{ActorActions, ActorKindActions, ActorMetadata, ActorObject};
use quantized::QuantizedSpec;
use serde::Deserialize;

pub mod e5;
pub mod gemma;
pub mod quantized;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorKind {
    Gemma(ActorObject<GemmaSpec>),
    Quantized(ActorObject<QuantizedSpec>),
    E5(ActorObject<E5Spec>),
}

impl ActorKindActions for ActorKind {
    fn actor(&self) -> Box<dyn ActorActions> {
        match self {
            ActorKind::Gemma(object) => Box::new(object.spec()),
            ActorKind::Quantized(object) => Box::new(object.spec()),
            ActorKind::E5(object) => Box::new(object.spec()),
        }
    }

    fn metadata(&self) -> ActorMetadata {
        match self {
            ActorKind::Gemma(object) => object.metadata(),
            ActorKind::Quantized(object) => object.metadata(),
            ActorKind::E5(object) => object.metadata(),
        }
    }
}
