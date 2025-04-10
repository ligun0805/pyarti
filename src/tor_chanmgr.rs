use std::sync::{Arc, Mutex};
use futures::stream::BoxStream;
use anyhow::{anyhow, Result as AnyResult};
use postage::broadcast::{self, Receiver, Sender};

use tor_rtcompat::Runtime;
use tor_memquota::{MemoryQuotaTracker, Config};
use tor_chanmgr::{ChanMgr, ChannelConfig, Dormancy};
use tor_netdir::{NetDir, NetDirProvider, DirEvent, Timeliness, Error, params::NetParameters};

pub struct TorChannelManager<R: Runtime> {
    chan_mgr: Arc<ChanMgr<R>>,
    dir_provider: Arc<CustomNetDirProvider>,
    runtime: R,
}

impl<R: Runtime> TorChannelManager<R> {
    pub fn new(runtime: R) -> AnyResult<Self> {
        let netparams = NetParameters::default();
        let chanmgr_config = ChannelConfig::default();
        let memquota_config = Config::builder()
            .build()?;
        let memquota_tracker = MemoryQuotaTracker::new(&runtime.clone(), memquota_config)?;

        let dir_provider = Arc::new(CustomNetDirProvider::new());

        let chan_mgr = Arc::new(ChanMgr::new(
            runtime.clone(),
            &chanmgr_config,
            Dormancy::Active,
            &netparams,
            memquota_tracker.clone(),
        ));

        Ok(Self {
            chan_mgr,
            runtime,
            dir_provider,
        })
    }

    pub fn init(&self, netdir: &NetDir) -> AnyResult<()> {
        self.dir_provider.set_netdir(netdir.clone());

        self.chan_mgr
            .launch_background_tasks(&self.runtime, self.dir_provider.clone())
            .map_err(|e| anyhow!("Failed to launch background tasks: {}", e))?;

        Ok(())
    }

    pub fn netdir(&self) -> AnyResult<Arc<NetDir>> {
        let netdir = self.dir_provider.netdir(Timeliness::Timely)
            .map_err(|e| anyhow!("Failed to get network directory: {}", e))?;

        Ok(netdir)
    }

    pub fn get_chanmgr(&self) -> AnyResult<Arc<ChanMgr<R>>> {
        Ok(self.chan_mgr.clone())
    }
}



pub type TResult<T> = std::result::Result<T, Error>;
pub struct CustomNetDirProvider {
    inner: Mutex<Inner>,
}

struct Inner {
    /// Current network directory
    current: Option<Arc<NetDir>>,
    /// Event sender for network directory updates
    event_tx: Sender<DirEvent>,
    /// Event receiver (kept to prevent channel closure)
    _event_rx: Receiver<DirEvent>,  
}

impl CustomNetDirProvider {
    pub fn new() -> Self {
        let (event_tx, _event_rx) = broadcast::channel(128);
        let inner = Inner {
            current: None,
            event_tx,
            _event_rx,
        };

        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn set_netdir(&self, dir: impl Into<Arc<NetDir>>) {
        let mut inner = self.inner.lock().expect("lock poisoned");
        inner.current = Some(dir.into());
    }
}

impl Default for CustomNetDirProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NetDirProvider for CustomNetDirProvider {
    fn netdir(&self, _timeliness: Timeliness) -> TResult<Arc<NetDir>> {
        match self.inner.lock().expect("lock poisoned").current.as_ref() {
            Some(netdir) => Ok(Arc::clone(netdir)),
            None => Err(tor_netdir::Error::NoInfo),
        }
    }

    fn events(&self) -> BoxStream<'static, DirEvent> {
        let inner = self.inner.lock().expect("lock poisoned");
        let events = inner.event_tx.subscribe();
        Box::pin(events)
    }

    fn params(&self) -> Arc<dyn AsRef<NetParameters>> {
        if let Ok(nd) = self.netdir(Timeliness::Unchecked) {
            nd
        } else {
            Arc::new(NetParameters::default())
        }
    }
}