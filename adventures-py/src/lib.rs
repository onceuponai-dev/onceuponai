extern crate adventures as adventures_rs;
use adventures_rs::{E5Model, E5_MODEL_REPO};
use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "adventures")]
fn adventures(_py: Python, m: &PyModule) -> PyResult<()> {
    #[pyclass]
    pub struct Adventures {
        model: E5Model,
    }

    #[pymethods]
    impl Adventures {
        #[new]
        pub fn new(e5_model_repo: Option<String>) -> Self {
            let e5_model_repo = if let Some(repo) = e5_model_repo {
                repo
            } else {
                E5_MODEL_REPO.to_string()
            };
            let model = E5Model::load(&e5_model_repo, "cpu").unwrap();
            Self { model }
        }

        pub fn embeddings(&self, input: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
            let embeddings = self.model.forward(input).unwrap();
            Ok(embeddings)
        }
    }

    impl Default for Adventures {
        fn default() -> Self {
            Self::new(Some(E5_MODEL_REPO.to_string()))
        }
    }

    m.add_class::<Adventures>()?;
    Ok(())
}
