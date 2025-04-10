mod tor_circmgr;
mod tor_chanmgr;
mod tor_hs_client;
mod tor_hs_connector;

use tor_circmgr::TorCircuitManager;
use tor_rtcompat::{BlockOn, PreferredRuntime};
use tor_hs_client::TorHSClient;

use log::info;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::collections::HashMap;
use futures::{AsyncReadExt, AsyncWriteExt};


#[pyclass]
#[pyo3(text_signature = "()")]
pub struct PyArtiClient {
    runtime: PreferredRuntime,
    circ_manager: TorCircuitManager<PreferredRuntime>,
}

#[pymethods]
impl PyArtiClient {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = PreferredRuntime::create()?;
        let circ_manager = TorCircuitManager::new(runtime.clone())
        .map_err(|e| PyValueError::new_err(format!("Failed to create circuit manager: {}", e)))?;

        Ok(Self { runtime, circ_manager })
    }

    #[pyo3(text_signature = "()")]
    fn init(&self) -> PyResult<()> {
        self.runtime.block_on(async {
            self.circ_manager.init().await
                .map_err(|e| PyValueError::new_err(format!("Initialization failed: {}", e)))
        })
    }

    #[pyo3(text_signature = "(relay_ip, relay_port, rsa_id)")]
    fn create(
        &mut self,
        relay_ip: &str,
        relay_port: u16,
        rsa_id: &str,
    ) -> PyResult<()> {
        self.runtime.block_on(async {
            match self.circ_manager.create(
                relay_ip,
                relay_port,
                rsa_id,
            ).await {
                Ok(_) => {
                    info!("Created the firsthop circuit.");

                    Ok(())
                },
                Err(e) => Err(PyValueError::new_err(format!("Connection failed: {}", e)))
            }
        })
    }

    #[pyo3(text_signature = "(relay_ip, relay_port, rsa_id)")]
    fn extend(
        &mut self,
        relay_ip: &str,
        relay_port: u16,
        rsa_id: &str,
    ) -> PyResult<()> {
        self.runtime.block_on(async {
            match self.circ_manager.extend(
                relay_ip,
                relay_port,
                rsa_id,
            ).await {
                Ok(_) => {
                    info!("Extended the circuit.");

                    Ok(())
                },
                Err(e) => Err(PyValueError::new_err(format!("Connection failed: {}", e)))
            }
        })
    }

    #[pyo3(text_signature = "(url, port)")]
    fn connect(&self, url: &str, port: u16) -> PyResult<String> {
        let (_, rest) = url.split_once("://")
            .ok_or_else(|| PyValueError::new_err("Invalid URL: Missing scheme (http or https)"))?;
    
        let (host, path) = match rest.split_once('/') {
            Some((host, path)) => (host, format!("/{}", path)),
            None => (rest, "/".to_string()),
        };
    
        self.runtime.block_on(async {
            let client_circ = self.circ_manager.get_circ()
                .map_err(|_| PyValueError::new_err("No circuit exists"))?;
    
            let request = format!(
                "GET {} HTTP/1.1\r\n\
                    Host: {}\r\n\
                    Connection: close\r\n\
                    \r\n",
                path, host
            );
    
            let mut stream = match client_circ.begin_stream(host, port, None).await {
                Ok(stream) => stream,
                Err(e) => return Err(PyValueError::new_err(format!("Failed to begin stream: {}", e))),
            };
    
            // Write request to the stream
            stream.write_all(request.as_bytes()).await.map_err(|e| {
                PyValueError::new_err(format!("Connection failed to write request: {}", e))
            })?;
    
            // IMPORTANT: Make sure the request was written.
            // Arti buffers data, so flushing the buffer is usually required.
            stream.flush().await.map_err(|e| {
                PyValueError::new_err(format!("Failed to flush stream: {}", e))
            })?;
    
            // Read the response into a string
            let mut response = String::new();
            match stream.read_to_string(&mut response).await {
                Ok(_) => Ok(response),
                Err(e) => Err(PyValueError::new_err(format!("Failed to read response: {}", e))),
            }
        })
    }
}

#[pyclass]
#[pyo3(text_signature = "()")]
pub struct PyArtiHSClient {
    runtime: PreferredRuntime,
    hs_client: TorHSClient,
}

#[pymethods]
impl PyArtiHSClient {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = PreferredRuntime::create()?;
        let hs_client = TorHSClient::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create tor hs_client: {}", e)))?;

        Ok(Self {
            runtime,
            hs_client
        })
    }

    #[pyo3(text_signature = "()")]
    fn init(&mut self,  storage: Option<HashMap<String, String>>) -> PyResult<()> {
        self.runtime.block_on(async {
            let storage_ref = storage.as_ref().map(|s| s);
            self.hs_client.init(storage_ref).await
                .map_err(|e| PyValueError::new_err(format!("Initialization failed: {}", e)))
        })
    }

    #[pyo3(text_signature = "()")]
    fn set_custom_hs_relay_ids(
        &self,
        guard_rsa_id: &str,
        middle_rsa_id: &str,
        exit_rsa_id: &str,
    ) -> PyResult<()> {
        self.hs_client.set_custom_hs_relay_ids(
            guard_rsa_id,
            middle_rsa_id,
            exit_rsa_id,
        ).map_err(|e| PyValueError::new_err(format!("Failed to set custom relay ids: {}", e)))?;

        Ok(())
    }

    #[pyo3(text_signature = "(hs_addr, hs_port)")]
    fn connect(&self, hs_addr: &str, hs_port: u16) -> PyResult<String> {
        self.runtime.block_on(async {
            self.hs_client.connect_to_hs(hs_addr, hs_port).await
                .map_err(|e| PyValueError::new_err(format!("Request failed failed: {}", e)))
        })
    }
}


#[pymodule]
fn pyarti(_py: Python, m: &PyModule) -> PyResult<()> {
    env_logger::init();
    m.add_class::<PyArtiClient>()?;
    m.add_class::<PyArtiHSClient>()?;
    m.add("__all__", vec!["PyArtiClient", "PyArtiHSClient"])?;
    Ok(())
}