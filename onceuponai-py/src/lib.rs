pub mod agents;
pub mod common;
pub mod llm;
extern crate onceuponai_core;
use common::ResultExt;
use llm::{Gemma, Quantized, E5};
use onceuponai_core::auth::validate_jwt_py;
use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "onceuponai")]
fn onceuponai(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let embeddings_module = PyModule::new_bound(py, "embeddings")?;
    embeddings_module.add_class::<E5>()?;

    m.add_submodule(&embeddings_module)?;

    let llms_module = PyModule::new_bound(py, "llms")?;
    llms_module.add_class::<Quantized>()?;
    llms_module.add_class::<Gemma>()?;

    m.add_submodule(&llms_module)?;

    let bot_module = PyModule::new_bound(py, "bot")?;

    bot_module.add_function(wrap_pyfunction!(validate_jwt, &bot_module)?)?;
    m.add_submodule(&bot_module)?;

    Ok(())
}

#[pyfunction]
async fn validate_jwt(token: String, jwks: String) -> PyResult<bool> {
    validate_jwt_py(&token, &jwks).await.map_pyerr()
}
