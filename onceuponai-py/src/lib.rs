extern crate onceuponai as onceuponai_rs;
use futures::channel::oneshot;
use onceuponai_rs::{
    common::OptionToResult,
    llm::{
        e5::{E5Model, E5_MODEL_REPO},
        quantized::QuantizedModel,
    },
};
use pyo3::{exceptions::PyTypeError, prelude::*};
use std::{thread, time::Duration};

#[pymodule]
#[pyo3(name = "onceuponai")]
fn onceuponai(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let embeddings_module = PyModule::new_bound(py, "embeddings")?;
    embeddings_module.add_class::<E5>()?;

    m.add_submodule(&embeddings_module)?;
    m.add_function(wrap_pyfunction!(sleep, m)?)?;

    let llms_module = PyModule::new_bound(py, "llms")?;
    llms_module.add_class::<Quantized>()?;

    m.add_submodule(&llms_module)?;
    Ok(())
}

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
        let device = if let Some(d) = device {
            d
        } else {
            "cpu".to_string()
        };
        let model = E5Model::load(&e5_model_repo, &device).map_pyerr()?;
        Ok(Self { model })
    }

    pub fn embed(&self, input: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
        let embeddings = self.model.forward(input).map_pyerr()?;
        Ok(embeddings)
    }
}

#[pyclass]
pub struct Quantized {
    model: QuantizedModel,
    eos_token: u32,
}

#[pymethods]
impl Quantized {
    #[new]
    pub fn new(
        model_repo: String,
        model_file: String,
        tokenizer_repo: String,
        device: String,
    ) -> PyResult<Self> {
        let model =
            QuantizedModel::load(&model_repo, &model_file, &tokenizer_repo, &device).map_pyerr()?;

        let eos_token = "</s>";
        let vocab = model.tokenizer.get_vocab(true).clone();
        let eos_token = *vocab.get(eos_token).ok_or_err("EOS_TOKEN").unwrap();

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

#[pyfunction]
async fn sleep(seconds: f64, result: Option<PyObject>) -> Option<PyObject> {
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs_f64(seconds));
        tx.send(()).unwrap();
    });
    rx.await.unwrap();
    result
}

pub trait ResultExt<T, E> {
    fn map_pyerr(self) -> Result<T, PyErr>;
}

impl<T, E: std::fmt::Debug> ResultExt<T, E> for Result<T, E> {
    fn map_pyerr(self) -> Result<T, PyErr> {
        self.map_err(|e| {
            let err = format!("{:?}", e);
            PyErr::new::<PyTypeError, _>(err)
        })
    }
}
