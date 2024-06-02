extern crate onceuponai_core;
use crate::common::ResultExt;
use onceuponai_core::{
    common::OptionToResult,
    llm::{
        e5::{E5Model, E5_MODEL_REPO},
        gemma::GemmaModel,
        quantized::QuantizedModel,
    },
};
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
}

#[pymethods]
impl Quantized {
    #[allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        model_repo: String,
        model_file: String,
        tokenizer_repo: Option<String>,
        device: Option<String>,
        seed: Option<u64>,
        repeat_last_n: Option<usize>,
        repeat_penalty: Option<f32>,
        temp: Option<f64>,
        top_p: Option<f64>,
    ) -> PyResult<Self> {
        let model =
            QuantizedModel::load(&model_repo, &model_file, tokenizer_repo, device).map_pyerr()?;

        let eos_token = "</s>";
        let vocab = model.tokenizer.get_vocab(true).clone();
        let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN").unwrap();

        Ok(Self {
            model,
            eos_token,
            seed,
            repeat_last_n,
            repeat_penalty,
            temp,
            top_p,
        })
    }

    pub async fn invoke(&mut self, prompt: String, sample_len: usize) -> PyResult<String> {
        let result = self
            .model
            .invoke(
                &prompt,
                sample_len,
                self.eos_token,
                self.seed,
                self.repeat_last_n,
                self.repeat_penalty,
                self.temp,
                self.top_p,
            )
            .await
            .map_pyerr()?;
        Ok(result)
    }
}

#[pyclass]
pub struct Gemma {
    model: GemmaModel,
    eos_token: u32,
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
        )
        .map_pyerr()?;

        let eos_token = match model.tokenizer.get_vocab(true).get("<eos>").copied() {
            Some(token) => token,
            None => {
                return Err(anyhow::anyhow!("EOS token not found in vocabulary")).map_pyerr()?
            }
        };

        Ok(Self { model, eos_token })
    }

    pub async fn invoke(&mut self, prompt: String, sample_len: usize) -> PyResult<String> {
        let result = self
            .model
            .invoke(&prompt, sample_len, self.eos_token)
            .await
            .map_pyerr()?;
        Ok(result)
    }
}
