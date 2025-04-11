use anyhow::Result as AnyResult;
use log::info;
use rustls::ServerName;
use std::{collections::HashMap, sync::Arc};

use arti_client::config::TorClientConfigBuilder;
use arti_client::{DataStream, StreamPrefs, TorClient, TorClientConfig};
use tor_circmgr::path::CustomHSRelaySetting;
use tor_linkspec::{HasAddrs, HasRelayIds};
use tor_llcrypto::pk::rsa::RsaIdentity;
use tor_netdir::Relay;
use tor_netdoc::doc::netstatus::RelayFlags;
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
                .create_unbootstrapped()?,
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
        // Set IPv6 as preferred to prioritize IPv6 connections when available
        s_prefs.ipv6_preferred();

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
                        "Relay {}: {}:{} ",
                        id,
                        if c_relay.addrs().len()>1 {
                            c_relay.addrs()[1].ip()
                        } else {
                            c_relay.addrs()[0].ip()
                        },
                        if c_relay.addrs().len()>1 {
                            c_relay.addrs()[1].port()
                        } else {
                            c_relay.addrs()[0].port()
                        }
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

    pub async fn select_relays(
        &self,
        relay_flags: u32,
        ipv6_required: bool,
        offset: usize,
        limit: i32,
    ) -> AnyResult<Vec<String>> {
        let arti_client = self
            .arti_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Arti client not initialized"))?;

        let netdir = arti_client.dirmgr().timely_netdir().unwrap();

        // Filter relays based on criteria
        let matching_relays: Vec<Relay<'_>> = netdir
            .relays()
            .filter(|relay| {
                // Check if the relay has all the required flags
                let has_required_flags = relay
                    .rs()
                    .flags()
                    .contains(self.u32_to_relay_flags(relay_flags));

                // Check IPv6 support if required
                let has_ipv6 = !ipv6_required || relay.addrs().len() > 1;

                has_required_flags && has_ipv6
            })
            .collect();

        println!("matching relays count : {}", matching_relays.len());
        let start = offset;
        let end = if limit < 0 {
            matching_relays.len()
        } else {
            std::cmp::min(start + (limit as usize), matching_relays.len())
        };

        let result: Vec<String> = matching_relays[start..end]
            .iter()
            .map(|relay|hex::encode(relay.rsa_id().as_bytes()).to_string())
            // .map(|relay| {
            //     format!(
            //         "{}",
            //         if relay.addrs().len() == 2 {
            //             format!("{}, {}", relay.addrs()[0].ip().to_string(), relay.addrs()[1].ip().to_string())
            //         } else {
            //             relay.addrs()[0].ip().to_string()
            //         }
            //     )
            // })
            .collect();

        Ok(result)
    }

    fn u32_to_relay_flags(&self, flags_u32: u32) -> RelayFlags {
        let mut relay_flags = RelayFlags::empty();

        // Map each bit to the corresponding flag
        // These mappings need to be adjusted based on the actual RelayFlags implementation
        if (flags_u32 & 0x0001) != 0 {
            relay_flags |= RelayFlags::AUTHORITY;
        }
        if (flags_u32 & 0x0002) != 0 {
            relay_flags |= RelayFlags::BAD_EXIT;
        }
        if (flags_u32 & 0x0004) != 0 {
            relay_flags |= RelayFlags::EXIT;
        }
        if (flags_u32 & 0x0008) != 0 {
            relay_flags |= RelayFlags::FAST;
        }
        if (flags_u32 & 0x0010) != 0 {
            relay_flags |= RelayFlags::GUARD;
        }
        if (flags_u32 & 0x0020) != 0 {
            relay_flags |= RelayFlags::HSDIR;
        }
        if (flags_u32 & 0x0040) != 0 {
            relay_flags |= RelayFlags::MIDDLE_ONLY;
        }
        if (flags_u32 & 0x0080) != 0 {
            relay_flags |= RelayFlags::NO_ED_CONSENSUS
        }
        if (flags_u32 & 0x0100) != 0 {
            relay_flags |= RelayFlags::STABLE;
        }
        if (flags_u32 & 0x0200) != 0 {
            relay_flags |= RelayFlags::STALE_DESC;
        }
        if (flags_u32 & 0x0400) != 0 {
            relay_flags |= RelayFlags::RUNNING;
        }
        if (flags_u32 & 0x0800) != 0 {
            relay_flags |= RelayFlags::VALID;
        }
        if (flags_u32 & 0x1000) != 0 {
            relay_flags |= RelayFlags::V2DIR;
        }
        // Add any other flags that exist in the library

        relay_flags
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
