use crate::tor_circmgr::TorCircuitManager;

use log::info;
use anyhow::{anyhow, Result as AnyResult};
use futures::{AsyncReadExt, AsyncWriteExt};
use tor_rtcompat::PreferredRuntime;

struct TorClient {
    circ_manager: TorCircuitManager<PreferredRuntime>,
}

impl TorClient {
    async fn new() -> AnyResult<Self> {
        let runtime = PreferredRuntime::current()?;
        let circ_manager = TorCircuitManager::new(runtime)?;

        circ_manager.init().await?;
        
        Ok(Self {
            circ_manager
        })
    }

    /// Perform a simple HTTP GET request
    async fn http_get(&mut self, host: &str, path: &str) -> AnyResult<String> {
        self.circ_manager.create(
            "88.198.35.49",
            443,
            "ED9A731373456FA071C12A3E63E2C8BEF0A6E721",
        ).await?;

        self.circ_manager.extend(
            "38.152.218.16",
            443,
            "B2708B9EFA3288656DFA9750B0FB926EB811EA77",
        ).await?;

        self.circ_manager.extend(
            "185.220.100.241",
            9000,
            "62F4994C6F3A5B3E590AEECE522591696C8DDEE2",
        ).await?;

        info!("Client circuit created successfully");

        let client_circ = self.circ_manager.get_circ()?;

        let mut stream = match client_circ.begin_stream(host, 80, None).await {
            Ok(stream) => stream,
            Err(e) => return Err(anyhow!("Failed to begin stream: {}", e)),
        };

        // Construct HTTP request
        let request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: close\r\n\
             \r\n",
            path, host
        );

        // Send the request
        stream.write_all(request.as_bytes()).await?;

        // IMPORTANT: Make sure the request was written.
        // Arti buffers data, so flushing the buffer is usually required.
        stream.flush().await?;

        // Read and print the result.
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;

        info!("{}", String::from_utf8_lossy(&buf));

        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    /// Fetch a specific URL through Tor
    async fn fetch_url(&mut self, url: &str) -> AnyResult<String> {
        // Parse URL to get host and path
        let url_parts: Vec<&str> = url.split("://").nth(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid URL"))?
            .splitn(2, '/').collect();
        
        let host = url_parts[0];
        let path = if url_parts.len() > 1 {
            format!("/{}", url_parts[1])
        } else {
            "/".to_string()
        };

        self.http_get(host, &path).await
    }
}

#[allow(dead_code)]
pub async fn test_tor_client() -> AnyResult<()> {
    let mut client = TorClient::new().await?;

    info!("Fetching example.com homepage...");
    match client.fetch_url("https://example.com").await {
        Ok(response) => {
            info!("Received response from example.com:");
            info!("{}", response);
        }
        Err(e) => info!("Error fetching example.com: {}", e),
    }
    
    Ok(())
}


/*******************************Test in Rust**********************************************
 * This script demonstrates how to create and manage a Tor circuit using the Arti library.
 * It initializes a Tor circuit, extends it with multiple relays, and performs an HTTP 
 * GET request over the established circuit.
******************************************************************************************

mod tor_circmgr;
mod tor_chanmgr;

use tor_circmgr::TorCircuitManager;
use tor_rtcompat::{BlockOn, PreferredRuntime};

use futures::{AsyncReadExt, AsyncWriteExt};
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;


#[pyclass]
#[pyo3(text_signature = "()")]
pub struct PyArti {
    runtime: PreferredRuntime,
    circ_manager: TorCircuitManager<PreferredRuntime>,
}

#[pymethods]
impl PyArti {
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

#[pymodule]
fn pyarti(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyArti>()?;
    m.add("__all__", vec!["PyArti"])?;
    Ok(())
}

******************************************************************************************/