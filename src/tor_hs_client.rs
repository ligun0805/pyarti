use crate::tor_hs_connector::{TorHSConnector, OnionCertificateVerifier};

use log::info;
use std::sync::Arc;
use std::time::Duration;
use std::convert::TryFrom;
use std::collections::HashMap;
use anyhow::{anyhow, Result as AnyResult};
use rustls::{ClientConfig, ServerName};
use tokio_rustls::TlsConnector;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct TorHSClient {
    hs_client: TorHSConnector,
}

impl TorHSClient {
    pub fn new() -> AnyResult<Self> {
        let hs_client = TorHSConnector::new()?;

        Ok(Self {
            hs_client
        })
    }

    pub async fn init(&mut self, storage: Option<&HashMap<String, String>>) -> AnyResult<()> {
        self.hs_client.init(storage).await
    }

    #[allow(dead_code)]
    pub fn set_custom_hs_relay_ids(
        &self,
        guard_rsa_id: &str,
        middle_rsa_id: &str,
        exit_rsa_id: &str,
    ) -> AnyResult<()> {
        self.hs_client.set_custom_hs_relay_ids(vec![
            guard_rsa_id.to_string(),
            middle_rsa_id.to_string(),
            exit_rsa_id.to_string(),
        ]);

        Ok(())
    }

    pub async fn connect_to_hs(&self, hs_addr: &str, hs_port: u16) -> AnyResult<String> {
        // Create a new stream to the hidden service
        let tcp_stream = match self.hs_client.connect_to_hs(hs_addr, hs_port).await {
            Ok(stream) => stream,
            Err(e) => return Err(anyhow!("Failed to begin stream: {}", e)),
        };

        if hs_port == 443 {
            // For HTTPS, we need a TLS connection
            return self.handle_https_connection(tcp_stream, hs_addr).await;
        } else {
            return self.handle_http_connection(tcp_stream, hs_addr).await;
        }
    }

    async fn handle_https_connection<S>(&self, tcp_stream: S, hs_addr: &str) -> AnyResult<String> 
    where 
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static
    {
        // Create a client config that will accept any certificate for .onion domains
        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(OnionCertificateVerifier {}))
            .with_no_client_auth();
            
        let tls_connector = TlsConnector::from(Arc::new(client_config));
        
        // Convert hostname to DNS name for TLS
        let dns_name = ServerName::try_from(hs_addr)
            .map_err(|_| anyhow!("Invalid DNS name: {}", hs_addr))?;
            
        // Establish TLS connection
        let mut tls_stream = tls_connector.connect(dns_name, tcp_stream).await
            .map_err(|e| anyhow!("TLS connection failed: {}", e))?;
            
        // Send HTTPS request
        let request = format!(
            "GET / HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: close\r\n\r\n",
            hs_addr
        );

        tls_stream.write_all(request.as_bytes()).await?;
        tls_stream.flush().await?;

        // Read response
        let mut response = Vec::new();
        let mut buffer = [0u8; 1024];
        
        // Read with timeout to avoid hanging indefinitely
        let timeout = Duration::from_secs(20);
        let mut total_bytes = 0;
        
        loop {
            let read_future = tls_stream.read(&mut buffer);
            let read_result = tokio::time::timeout(timeout, read_future).await;
            
            match read_result {
                Ok(Ok(0)) => break, // End of stream
                Ok(Ok(n)) => {
                    response.extend_from_slice(&buffer[..n]);
                    total_bytes += n;
                    info!("Received {} bytes (total: {})", n, total_bytes);
                    
                    // Limit the response size to avoid excessive memory usage
                    if total_bytes > 10 * 1024 * 1024 {
                        info!("Response exceeds 10MB, truncating");
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("Error reading from stream: {}", e));
                }
                Err(_) => {
                    return Err(anyhow!("Read operation timed out"));
                }
            }
        }
        
        // Display the response (first 1000 bytes)
        let response_str = String::from_utf8_lossy(&response[..std::cmp::min(1000, response.len())]);
        info!("Total response size: {} bytes", response.len());
        
        Ok(response_str.to_string())
    }
    
    async fn handle_http_connection<S>(&self, mut stream: S, hs_addr: &str) -> AnyResult<String> 
    where 
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin
    {
        let request = format!(
            "GET / HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: close\r\n\r\n",
            hs_addr
        );

        stream.write_all(request.as_bytes()).await?;
        stream.flush().await?;

        let mut response = Vec::new();
        let mut buffer = [0u8; 1024];
        
        // Read with timeout to avoid hanging indefinitely
        let timeout = Duration::from_secs(20);
        let mut total_bytes = 0;
        
        loop {
            let read_future = stream.read(&mut buffer);
            let read_result = tokio::time::timeout(timeout, read_future).await;
            
            match read_result {
                Ok(Ok(0)) => break, // End of stream
                Ok(Ok(n)) => {
                    response.extend_from_slice(&buffer[..n]);
                    total_bytes += n;

                    info!("Received {} bytes (total: {})", n, total_bytes);
                    
                    // Limit the response size to avoid excessive memory usage
                    if total_bytes > 10 * 1024 * 1024 {
                        info!("Response exceeds 10MB, truncating");
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("Error reading from stream: {}", e));
                }
                Err(_) => {
                    return Err(anyhow!("Read operation timed out"));
                }
            }
        }
        
        let response_str = String::from_utf8_lossy(&response[..std::cmp::min(1000, response.len())]);
        info!("Total response size: {} bytes", response.len());
        
        Ok(response_str.to_string())
    }
}