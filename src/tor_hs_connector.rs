use anyhow::Result as AnyResult;
use log::info;
use rustls::ServerName;
use std::{collections::HashMap, sync::Arc};

use arti_client::config::TorClientConfigBuilder;
use arti_client::{DataStream, StreamPrefs, TorClient, TorClientConfig};
use tor_circmgr::path::CustomHSRelaySetting;
use tor_linkspec::HasAddrs;
use tor_llcrypto::pk::rsa::RsaIdentity;
use tor_rtcompat::PreferredRuntime;

pub struct TorHSConnector {
    arti_client: Option<Arc<TorClient<PreferredRuntime>>>,
}

impl TorHSConnector {
    pub fn new() -> AnyResult<Self> {
        Ok(Self { arti_client: None })
    }

    pub async fn init(&mut self, storage: Option<&HashMap<String, String>>) -> AnyResult<()> {
        let config = if let Some(storage_map) = storage {
            let state_dir = storage_map.get("state_dir").unwrap();
            let cache_dir = storage_map.get("cache_dir").unwrap();

            //  Load config from cache
            TorClientConfigBuilder::from_directories(state_dir, cache_dir)
                .build()
                .unwrap()
        } else {
            TorClientConfig::default()
        };

        let arti_client = Arc::new(
            TorClient::builder()
            .config(config)
            .create_unbootstrapped()?
        );
        
        info!("load directory from cache");
        arti_client.load_cache().await?;
        if !arti_client.dirmgr().timely_netdir().is_ok() {
            info!("bootstrap manually");
            arti_client.bootstrap().await?;
        }

        self.arti_client = Some(arti_client);

        Ok(())
    }

    pub fn set_custom_hs_relay_ids(&self, rsa_ids: Vec<String>) {
        CustomHSRelaySetting::set(rsa_ids);
    }

    pub async fn connect_to_hs(&self, hs_addr: &str, hs_port: u16) -> AnyResult<DataStream> {
        let mut s_prefs = StreamPrefs::new();
        s_prefs.connect_to_onion_services(arti_client::config::BoolOrAuto::Explicit(true));

        let hs_addr = hs_addr.to_string();
        let arti_client = self
            .arti_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Arti client not initialized"))?;

        let relay_ids_str = CustomHSRelaySetting::get();
        if relay_ids_str.len() == 3 {
            info!("Connecting through the custom circuit:");

            for (id, rsa_id) in relay_ids_str.iter().enumerate() {
                let netdir = arti_client.dirmgr().timely_netdir().unwrap();
                let rsa_id_bytes = match hex::decode(rsa_id) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        info!("Invalid RSA fingerprint");
                        continue;
                    }
                };

                let rsa_identity = RsaIdentity::from_bytes(&rsa_id_bytes)
                    .expect("Failed to create RsaIdentity from bytes");

                if let Some(c_relay) = netdir.by_id(&rsa_identity) {
                    info!(
                        "Relay {}: {}:{}",
                        id,
                        c_relay.addrs()[0].ip(),
                        c_relay.addrs()[0].port()
                    );
                    if id == 2 {
                        info!("");
                    }
                }
            }
        } else {
            info!("Custom relays is not set. Connecting through the random circuit:");
        };

        let stream = arti_client
            .connect_with_prefs((hs_addr, hs_port), &s_prefs)
            .await?;

        Ok(stream)
    }
}

pub struct OnionCertificateVerifier {}

impl rustls::client::ServerCertVerifier for OnionCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // For .onion domains, accept any certificate
        if let ServerName::DnsName(dns_name) = server_name {
            if dns_name.as_ref().ends_with(".onion") {
                return Ok(rustls::client::ServerCertVerified::assertion());
            }
        }

        // For non-.onion domains, reject
        Err(rustls::Error::General(
            "Certificate verification failed".into(),
        ))
    }
}
