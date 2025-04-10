use crate::tor_chanmgr::TorChannelManager;

use log::info;
use std::sync::Arc;
use std::net::SocketAddr;
use futures::task::SpawnExt;
use anyhow::{anyhow, Result as AnyResult};

use arti_client::{TorClient, TorClientConfig};

use tor_rtcompat::Runtime;
use tor_units::Percentage;
use tor_llcrypto::pk::rsa::RsaIdentity;
use tor_chanmgr::{ChannelUsage, ChanProvenance};
use tor_linkspec::{ChanTarget, CircTarget, HasRelayIds, IntoOwnedChanTarget, OwnedChanTarget, OwnedCircTarget};
use tor_proto::circuit::{ClientCirc, PendingClientCirc, CircParameters};
use tor_proto::ccparams::{
    Algorithm, CongestionControlParamsBuilder, FixedWindowParamsBuilder,
    RoundTripEstimatorParamsBuilder, CongestionWindowParamsBuilder
};

pub struct TorCircuitManager<R: Runtime> {
    tor_chan_mgr: TorChannelManager<R>,
    circ: Option<Arc<ClientCirc>>,
    runtime: R,
}

impl<R: Runtime> TorCircuitManager<R> {
    async fn create_common<CT: ChanTarget>(
        &self,
        rt: &R,
        target: &CT,
        usage: ChannelUsage,
    ) -> AnyResult<PendingClientCirc> {
        let chanmgr = self.tor_chan_mgr.get_chanmgr()
            .map_err(|_| anyhow!("Failed to get channel manager"))?;
        let result = chanmgr.get_or_launch(target, usage).await;

        let chan = match result {
            Ok((chan, ChanProvenance::NewlyCreated)) => chan,
            Ok((chan, _)) => chan,
            Err(_) => return Err(anyhow!("Failed to get or launch channel")),
        };
        // Construct the (zero-hop) circuit.
        let (pending_circ, reactor) = chan.new_circ()
            .await.map_err(|_| anyhow!("Failed to create circuit"))?;

        rt.spawn(async {
            let _ = reactor.run().await;
        })
            .map_err(|_| anyhow!("Failed to spawn circuit reactor"))?;

        Ok(pending_circ)
    }

    fn rsa_key_from_fingerprint(&self, fingerprint: &str) -> AnyResult<RsaIdentity> {
        let rsa_id_bytes = hex::decode(fingerprint.replace(" ", ""))
            .map_err(|e| anyhow!("Invalid RSA fingerprint: {}", e))?;
        if rsa_id_bytes.len() != 20 {
            return Err(anyhow!("RSA fingerprint must be 20 bytes (40 hex characters)"));
        }
        let rsa_identity = RsaIdentity::from_bytes(&rsa_id_bytes)
            .expect("Failed to create RsaIdentity from bytes");

        Ok(rsa_identity.clone())
    }

    async fn circ_target_from_relay(
        &self, 
        relay_ip: &str,
        relay_port: u16,
        relay_fingerprint: &str,
    ) -> AnyResult<OwnedCircTarget> {
        let addr = format!("{}:{}", relay_ip, relay_port)
            .parse::<SocketAddr>()
            .map_err(|e| anyhow!("Invalid address: {}", e))?;

        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.runtime.connect(&addr)
        )
            .await
            .map_err(|_| anyhow!("Timeout connecting to relay"))?
            .map_err(|e| anyhow!("Failed to connect to relay {}:{}: {}", relay_ip, relay_port, e))?;

        info!("TCP connection established successfully to {}:{}", relay_ip, relay_port);

        // Get a relay by its RSA fingerprint
        let netdir = self.tor_chan_mgr.netdir()?;
        let rsa_identity = self.rsa_key_from_fingerprint(relay_fingerprint)?;
        let relay = netdir.by_id(&rsa_identity)
            .ok_or_else(|| anyhow!("Relay not found"))?;
        
        // Create channel target using the builder pattern
        let mut builder = OwnedCircTarget::builder();
        builder
            .chan_target()
            .addrs(vec![addr])
            .ed_identity(relay.ed_identity().unwrap().clone())
            .rsa_identity(rsa_identity.clone());
        let target = builder
            .ntor_onion_key(relay.ntor_onion_key().clone())
            .protocols("FlowCtrl=7".parse().unwrap())
            .build()
            .unwrap();

        Ok(target)
    }

    #[allow(dead_code)]
    async fn inner_create_one_hop(
        &self,
        ct: &OwnedChanTarget,
        params: &CircParameters,
        usage: ChannelUsage,
    ) -> AnyResult<Arc<ClientCirc>> {
        let circ = self.create_common(&self.runtime, ct, usage).await?;

        circ.create_firsthop_fast(params)
            .await
            .map_err(|_| anyhow!("Failed to create first hop: {}", ct.to_logged().to_string()))
    }

    async fn inner_create(
        &self,
        ct: &OwnedCircTarget,
        params: &CircParameters,
        usage: ChannelUsage,
    ) -> AnyResult<Arc<ClientCirc>> {
        let circ = self.create_common(&self.runtime, ct, usage).await?;

        let params = params.clone();
        let handshake_res = circ.create_firsthop_ntor(ct, params).await;

        handshake_res.map_err(|_| anyhow!("Failed to create first hop: {}", ct.to_logged().to_string()))
    }

    pub fn new(runtime: R) -> AnyResult<Self> {
        let tor_chan_mgr = TorChannelManager::new(runtime.clone())
            .map_err(|e| anyhow!("Failed to create channel manager: {}", e))?;

        Ok(Self {
            tor_chan_mgr,
            circ: None,
            runtime,
        })
    }

    pub async fn init(&self) -> AnyResult<()> {
        let config = TorClientConfig::default();
        let arti_client = Arc::new(TorClient::create_bootstrapped(config).await?);
        let netdir = arti_client.dirmgr().timely_netdir().unwrap();

        self.tor_chan_mgr.init(&netdir)
    }

    pub fn get_circ(&self) -> AnyResult<Arc<ClientCirc>> {
        match self.circ {
            Some(_) => {
                let circ = self.circ.as_ref().unwrap().clone();
                Ok(circ)
            },
            None => Err(anyhow!("No circuit to extend"))
        }
    }

    #[allow(dead_code)]
    pub async fn create_one_hop(
        &self, 
        relay_ip: &str,
        relay_port: u16,
        relay_fingerprint: &str
    ) -> AnyResult<Arc<ClientCirc>> {
        let mut circ_target = self.circ_target_from_relay(relay_ip, relay_port, relay_fingerprint).await?;
        let char_target = circ_target.chan_target_mut().clone();
        let cc_params = self.build_circuit_params()?;
        let circ_params = CircParameters::new(true, cc_params);

        let client_circ = self.inner_create_one_hop(&char_target, &circ_params, ChannelUsage::UserTraffic)
            .await?;

        Ok(client_circ)
    }

    pub async fn create(
        &mut self,
        relay_ip: &str,
        relay_port: u16,
        relay_fingerprint: &str
    ) -> AnyResult<Arc<ClientCirc>> {
        let circ_target = self.circ_target_from_relay(relay_ip, relay_port, relay_fingerprint)
            .await?;
        let cc_params = self.build_circuit_params()?;
        let circ_params = CircParameters::new(true, cc_params);

        let client_circ = self.inner_create(&circ_target, &circ_params, ChannelUsage::UserTraffic)
            .await?;

        self.circ = Some(client_circ.clone());

        Ok(client_circ)
    }

    pub async fn extend(
        &mut self,
        relay_ip: &str,
        relay_port: u16,
        relay_fingerprint: &str
    ) -> AnyResult<Arc<ClientCirc>> {
        match self.circ {
            Some(_) => {
                let circ = self.circ.as_ref().unwrap().clone();
                let circ_target = self.circ_target_from_relay(relay_ip, relay_port, relay_fingerprint)
                .await?;
                let cc_params = self.build_circuit_params()?;
                let circ_params = CircParameters::new(true, cc_params);

                circ.extend_ntor(&circ_target, &circ_params).await?;

                Ok(circ)
            },
            None => Err(anyhow!("No circuit to extend"))
        }
    }

    fn build_circuit_params(&self) -> AnyResult<tor_proto::ccparams::CongestionControlParams> {
        let params = FixedWindowParamsBuilder::default()
            .circ_window_start(1000)
            .circ_window_min(100)
            .circ_window_max(1000)
            .build()
            .map_err(|e| anyhow!("Failed to build fixed window params: {}", e))?;

        let rtt_params = RoundTripEstimatorParamsBuilder::default()
            .ewma_cwnd_pct(Percentage::new(50))
            .ewma_max(10)
            .ewma_ss_max(2)
            .rtt_reset_pct(Percentage::new(100))
            .build()
            .map_err(|e| anyhow!("Failed to build RTT parameters: {}", e))?;

        let cwnd_params = CongestionWindowParamsBuilder::default()
            .cwnd_init(124)
            .cwnd_inc_pct_ss(Percentage::new(100))
            .cwnd_inc(1)
            .cwnd_inc_rate(31)
            .cwnd_min(124)
            .cwnd_max(u32::MAX)
            .sendme_inc(31)
            .build()
            .map_err(|e| anyhow!("Failed to build congestion window parameters: {}", e))?;

        CongestionControlParamsBuilder::default()
            .rtt_params(rtt_params)
            .cwnd_params(cwnd_params)
            .alg(Algorithm::FixedWindow(params))
            .build()
            .map_err(|e| anyhow!("Failed to build CC params: {}", e))
    }
}