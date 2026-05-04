mod data;
mod index;
mod types;
mod updater_core;

use pyo3::prelude::*;

#[pymodule]
mod native {
    use std::fs::File;
    use std::io::{BufReader, BufWriter};

    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::PyTuple;

    use xz2::write::XzEncoder;

    use crate::types::Document;
    use crate::updater_core;

    #[pymodule]
    mod typings {
        use crate::types;

        #[pymodule_export]
        use types::{
            BasicAlliance, BasicPlayer, CastleTimers, CoatOfArms, Document, Faction, Location,
        };
    }

    #[pyclass]
    struct Updater {
        core: Option<updater_core::Updater<BufReader<File>, XzEncoder<BufWriter<File>>>>,
        input_filename: String,
        output_filename: String,
    }

    #[pymethods]
    impl Updater {
        #[new]
        fn new(input_filename: String, output_filename: String) -> Self {
            Updater {
                core: None,
                input_filename,
                output_filename,
            }
        }

        fn __enter__(&mut self, py: Python<'_>) -> PyResult<()> {
            let mut updater = updater_core::Updater::new();
            updater.set_input_buffer(BufReader::new(File::open(&*self.input_filename)?));
            updater.set_output_buffer(XzEncoder::new(
                BufWriter::new(File::create(&*self.output_filename)?),
                6,
            ));

            let mut result = Ok(());
            // Release GIL as init is CPU-bound
            py.detach(|| {
                result = updater.init();
            });
            result.map_err(|error| {
                PyRuntimeError::new_err(format!("Cannot initialize object: {error:?}"))
            })?;

            self.core = Some(updater); // Lazy initialization
            Ok(())
        }

        // Using *args due to difficulty implementing the typical signature of __exit__
        #[pyo3(signature = (*_args))]
        fn __exit__(&mut self, py: Python<'_>, _args: &Bound<'_, PyTuple>) -> PyResult<bool> {
            let core = self.core.as_mut().ok_or(PyValueError::new_err(
                "Cannot use object outside of context manager",
            ))?;

            let mut result = Ok(());
            // Release GIL
            py.detach(|| {
                result = core.finalize();
            });
            result.map_err(|error| {
                PyRuntimeError::new_err(format!("Cannot finalize object: {error:?}"))
            })?;

            self.core = None; // Remove reference
            Ok(false)
        }

        fn update(&mut self, py: Python<'_>, document: Document) -> PyResult<()> {
            let core = self.core.as_mut().ok_or(PyValueError::new_err(
                "Cannot use object outside of context manager",
            ))?;
            let mut result = Ok(());
            // Release GIL
            py.detach(|| {
                result = core.update(document);
            });
            result.map_err(|error| {
                PyRuntimeError::new_err(format!("Cannot update using object: {error:?}"))
            })
        }
    }
}
