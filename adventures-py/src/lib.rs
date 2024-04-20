extern crate adventures as adventures_rs;
use adventures_rs::E5Model;
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
        pub fn new() -> Self {
            let model = E5Model::load().unwrap();
            Self { model }
        }

        pub fn embeddings(&self, input: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
            let embeddings = self.model.forward(input).unwrap();
            Ok(embeddings)
        }
    }

    impl Default for Adventures {
        fn default() -> Self {
            Self::new()
        }
    }

    m.add_class::<Adventures>()?;
    Ok(())
}
