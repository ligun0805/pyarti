use crate::tor_hs_client::TorHSClient;
use tor_netdoc::doc::netstatus::RelayFlags;

use log::info;
use std::collections::HashMap;
use anyhow::Result as AnyResult;

#[allow(dead_code)]
pub async fn test_tor_hs_client() -> AnyResult<()> {
    info!("Preparing Tor Client...");
    let mut client = TorHSClient::new()?;
    // let mut client2 = TorHSClient::new()?;

    let mut storage = HashMap::new();
    storage.insert("cache_dir".to_string(), "/home/arti/cache".to_string());
    storage.insert("state_dir".to_string(), "/home/arti/state".to_string());

    client.init(Some(&storage)).await?;

    let flags = RelayFlags::EXIT | RelayFlags::FAST;
    let exit_flags = flags.bits();
    let exit_relays = client.select_relays(
        exit_flags.into(),
        true, // ipv6_required
        0,    // offset
        10    // limit - get first 10 relays
    ).await?;
    
    println!("\nFound {} EXIT relays with IPv6 support:", exit_relays.len());
    for (i, relay) in exit_relays.iter().enumerate() {
        println!("  {}: {}", i+1, relay);
    }

    // Set custom relays for the circuit from client to rendezvous point
    client.set_custom_hs_relay_ids(
        "FFA72BD683BC2FCF988356E6BEC1E490F313FB07",
        "00194c4feceb999cf8c7ef1d202ae9bf24d17545",
        "8929AF5554BE622DE3FE34812C03D65FE7D5D0F1",
    )?;
    
    let duckduckgo_addr = "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion";
    info!("Connecting to the hidden service: {}", duckduckgo_addr);

    match client.connect_to_hs(duckduckgo_addr, 443).await {
        Ok(_) => {
            info!("Received response from {}", duckduckgo_addr);
        }
        Err(e) => info!("Error fetching {}: {}", duckduckgo_addr, e),
    }
    
    Ok(())
}