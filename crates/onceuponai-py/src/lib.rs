pub mod agents;
pub mod common;
pub mod llm;
extern crate onceuponai_core;
use llm::{Gemma, Quantized, E5};
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

    Ok(())
}
