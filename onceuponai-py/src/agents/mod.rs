use pyo3::{prelude::*, types::PyDict};

pub trait AgentEngine {
    fn __call__(&mut self, messages: Vec<PyDict>) -> PyResult<String>;
}

// use crate::llm::Quantized;

// pub enum EngineModel {
//     Quantized(crate::llm::Quantized),
// }

// #[pyclass]
// pub struct CandleEngine {
//     model: EngineModel,
//     model_type: String,
// }

// #[pymethods]
// impl CandleEngine {
//     #[new]
//     pub fn new(
//         model_type: &str,
//         model_repo: String,
//         model_file: String,
//         model_revision: Option<String>,
//         tokenizer_repo: Option<String>,
//         device: Option<String>,
//         seed: Option<u64>,
//         repeat_last_n: Option<usize>,
//         repeat_penalty: Option<f32>,
//         temp: Option<f64>,
//         top_p: Option<f64>,
//         hf_token: Option<String>,
//         use_flash_attn: Option<bool>,
//     ) -> PyResult<Self> {
//         let model = match model_type {
//             "quantized" => EngineModel::Quantized(crate::llm::Quantized::new(
//                 model_repo,
//                 model_file,
//                 model_revision,
//                 tokenizer_repo,
//                 device,
//                 seed,
//                 repeat_last_n,
//                 repeat_penalty,
//                 temp,
//                 top_p,
//             )?),
//             _ => todo!("other models"),
//         };

//         Ok(Self {
//             model,
//             model_type: model_type.to_string(),
//         })
//     }

//     pub fn __call__(&mut self, prompt: String) -> PyResult<String> {
//         let mut result: String = match &self.model {
//             EngineModel::Quantized(model) => {
//                 let mut m = model;
//                 (*m.invoke(prompt, 200)?).to_string()
//             }
//             _ => todo!(),
//         };

//         Ok(result.to_string())
//     }
// }
