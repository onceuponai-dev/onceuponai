pub mod common;
use crate::common::{hf_hub_get, hf_hub_get_multiple, hf_hub_get_path, ResultExt};
use anyhow::Result;
use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use candle_transformers::models::gemma::{Config, Model};
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;
use tokenizers::{PaddingParams, Tokenizer};

pub struct GemmaModel {
    pub model: Model,
    pub device: Device,
    pub tokenizer: Tokenizer,
    pub logits_processor: LogitsProcessor,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl GemmaModel {
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        base_repo_id: &str,
        model_endpoint: Option<String>,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        repeat_penalty: f32,
        repeat_last_n: usize,
        hf_token: Option<String>,
    ) -> Result<GemmaModel> {
        let paths = hf_hub_get_multiple(
            base_repo_id,
            "model.safetensors.index.json",
            model_endpoint.clone(),
            hf_token.clone(),
        )?;

        let device = &Device::Cpu;
        // let device = &Device::new_cuda(0).unwrap();
        let dtype = if device.is_cuda() {
            DType::BF16
        } else {
            DType::F32
        };

        //let device = &Device::new_cuda(0)?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&paths, dtype, device)? };
        let tokenizer = hf_hub_get(
            base_repo_id,
            "tokenizer.json",
            model_endpoint.clone(),
            hf_token.clone(),
        )?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;
        let candle_config = hf_hub_get(
            base_repo_id,
            "config.json",
            model_endpoint.clone(),
            hf_token,
        )?;
        let candle_config: Config = serde_json::from_slice(&candle_config)?;
        let model = Model::new(&candle_config, vb)?;

        let logits_processor = LogitsProcessor::new(seed, temp, top_p);
        Ok(GemmaModel {
            model,
            tokenizer,
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            device: device.clone(),
        })
    }
}

pub struct GemmaState {
    pub eos_token: u32,
}

pub struct E5Model {
    pub model: BertModel,
    pub tokenizer: Tokenizer,
    pub normalize_embeddings: Option<bool>,
}

impl E5Model {
    pub fn load() -> Result<E5Model> {
        let base_repo_id = "intfloat/e5-small-v2";
        let weights = hf_hub_get(base_repo_id, "model.safetensors", None, None)?;
        let tokenizer = hf_hub_get(base_repo_id, "tokenizer.json", None, None)?;
        let candle_config = hf_hub_get(base_repo_id, "config.json", None, None)?;
        let candle_config: BertConfig = serde_json::from_slice(&candle_config)?;

        let device = &Device::Cpu;
        let mut tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;

        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, device)?;
        let model = BertModel::load(vb, &candle_config)?;
        Ok(E5Model {
            model,
            tokenizer,
            normalize_embeddings: Some(true),
        })
    }

    pub fn forward(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let device = &Device::Cpu;
        let tokens = self
            .tokenizer
            .encode_batch(input.clone(), true)
            .map_anyhow_err()?;

        let token_ids: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let token_type_ids = token_ids.zeros_like()?;

        let embeddings = self.model.forward(&token_ids, &token_type_ids)?;
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = if let Some(true) = self.normalize_embeddings {
            embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?
        } else {
            embeddings
        };
        let embeddings_data: Vec<Vec<f32>> = embeddings.to_vec2()?;
        Ok(embeddings_data)
    }
}

pub struct QuantizedModel {
    pub model: ModelWeights,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

impl QuantizedModel {
    pub fn load() -> Result<QuantizedModel> {
        //let base_repo_id = ("TheBloke/CodeLlama-7B-GGUF", "codellama-7b.Q4_0.gguf");
        let base_repo_id = (
            "TheBloke/Mistral-7B-Instruct-v0.2-GGUF",
            "mistral-7b-instruct-v0.2.Q4_K_S.gguf",
        );
        //let base_repo_id = ("MaziyarPanahi/gemma-2b-it-GGUF", "gemma-2b-it.Q4_K_M.gguf");
        //let tokenizer_repo = "hf-internal-testing/llama-tokenizer";
        let tokenizer_repo = "mistralai/Mistral-7B-Instruct-v0.2";
        //let tokenizer_repo = "google/gemma-2b-it";

        let model_path = hf_hub_get_path(base_repo_id.0, base_repo_id.1, None, None)?;
        let tokenizer = hf_hub_get(tokenizer_repo, "tokenizer.json", None, None)?;

        //let device = Device::Cpu;
        let device = Device::new_cuda(0).unwrap();
        let mut tokenizer = Tokenizer::from_bytes(&tokenizer).map_anyhow_err()?;

        let mut file = std::fs::File::open(&model_path)?;
        let model_content = gguf_file::Content::read(&mut file)?;
        let model = ModelWeights::from_gguf(model_content, &mut file, &device)?;

        Ok(QuantizedModel {
            model,
            tokenizer,
            device,
        })
    }
}
