extern crate onceuponai_core;
use std::collections::HashMap;

use crate::common::ResultExt;
use onceuponai_candle::llm::{
    e5::{E5Model, E5_MODEL_REPO},
    gemma::GemmaModel,
    quantized::QuantizedModel,
};
use onceuponai_core::common::OptionToResult;
use pyo3::prelude::*;

#[pyclass]
pub struct E5 {
    model: E5Model,
}

#[pymethods]
impl E5 {
    #[new]
    pub fn new(e5_model_repo: Option<String>, device: Option<String>) -> PyResult<Self> {
        let e5_model_repo = if let Some(repo) = e5_model_repo {
            repo
        } else {
            E5_MODEL_REPO.to_string()
        };
        let model = E5Model::load(&e5_model_repo, device).map_pyerr()?;
        Ok(Self { model })
    }

    pub fn embed(&self, input: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
        let embeddings = self.model.embed(input).map_pyerr()?;
        Ok(embeddings)
    }
}

#[pyclass]
pub struct Quantized {
    model: QuantizedModel,
    eos_token: u32,
    seed: Option<u64>,
    repeat_last_n: Option<usize>,
    repeat_penalty: Option<f32>,
    temp: Option<f64>,
    top_p: Option<f64>,
    sample_len: usize,
}

#[pymethods]
impl Quantized {
    #[allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        model_repo: String,
        model_file: String,
        model_revision: Option<String>,
        tokenizer_repo: Option<String>,
        device: Option<String>,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
        sample_len: Option<usize>,
    ) -> PyResult<Self> {
        let model = QuantizedModel::load(
            &model_repo,
            &model_file,
            model_revision,
            tokenizer_repo,
            device,
        )
        .map_pyerr()?;

        let eos_token = "</s>";
        let vocab = model.tokenizer.get_vocab(true).clone();
        let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN").map_pyerr()?;
        let sample_len = sample_len.unwrap_or(1000);

        Ok(Self {
            model,
            eos_token,
            seed,
            repeat_last_n,
            repeat_penalty,
            temp,
            top_p,
            sample_len,
        })
    }

    pub fn invoke(&mut self, prompt: String) -> PyResult<String> {
        let result = self
            .model
            .invoke(
                &prompt,
                self.sample_len,
                self.eos_token,
                self.seed,
                self.repeat_last_n,
                self.repeat_penalty,
                self.temp,
                self.top_p,
            )
            .map_pyerr()?;
        Ok(result)
    }

    fn __call__(
        &mut self,
        messages: Vec<HashMap<String, String>>,
        stop_sequences: Vec<String>,
    ) -> PyResult<String> {
        let mut prompt = String::new();
        let l = messages.len();

        for (i, dict) in messages.iter().enumerate() {
            let role = dict.get("role").expect("role");
            let content = dict.get("content").expect("content");
            if i < l {
                if role == "user" {
                    prompt.push_str(&format!("<s> [INST] {} [/INST] ", content));
                } else {
                    prompt.push_str(&format!(" {} </s>", content));
                }
            } else {
                prompt.push_str(&format!("[INST] {} [/INST] ", content));
            }
        }

        self.invoke(prompt)
    }
}

#[pyclass]
pub struct Gemma {
    model: GemmaModel,
    eos_token: u32,
    sample_len: usize,
}

#[pymethods]
impl Gemma {
    #[allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        model_repo: String,
        tokenizer_repo: Option<String>,
        device: Option<String>,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
        hf_token: Option<String>,
        use_flash_attn: Option<bool>,
        sample_len: Option<usize>,
    ) -> PyResult<Self> {
        let model = GemmaModel::load(
            model_repo,
            tokenizer_repo,
            device,
            seed,
            repeat_last_n,
            repeat_penalty,
            temp,
            top_p,
            hf_token,
            use_flash_attn,
        )
        .map_pyerr()?;

        let eos_token = match model.tokenizer.get_vocab(true).get("<eos>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_pyerr()?
            }
        };

        let sample_len = sample_len.unwrap_or(1000);

        Ok(Self {
            model,
            eos_token,
            sample_len,
        })
    }

    pub fn invoke(&mut self, prompt: String) -> PyResult<String> {
        let result = self
            .model
            .invoke(&prompt, self.sample_len, self.eos_token)
            .map_pyerr()?;
        Ok(result)
    }
}
