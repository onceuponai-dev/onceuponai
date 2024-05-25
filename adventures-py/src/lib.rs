extern crate adventures as adventures_rs;
use adventures_rs::llm::e5::{E5Model, E5_MODEL_REPO};
use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "onceuponai")]
fn onceuponai(_py: Python, m: &PyModule) -> PyResult<()> {
    #[pyclass]
    pub struct Adventures {
        model: E5Model,
    }

    #[pymethods]
    impl Adventures {
        #[new]
        pub fn new(e5_model_repo: Option<String>, device: Option<String>) -> Self {
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
            let model = E5Model::load(&e5_model_repo, &device).unwrap();
            Self { model }
        }

        pub fn embeddings(&self, input: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
            let embeddings = self.model.forward(input).unwrap();
            Ok(embeddings)
        }
    }

    impl Default for Adventures {
        fn default() -> Self {
            Self::new(Some(E5_MODEL_REPO.to_string()), None)
        }
    }

    m.add_class::<Adventures>()?;
    Ok(())
}
