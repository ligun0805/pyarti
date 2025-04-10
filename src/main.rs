mod tor_circmgr;
mod tor_chanmgr;
mod tor_hs_client;
mod tor_hs_connector;

mod test;

use anyhow::Result;

use test::tor_hs_client_test::test_tor_hs_client;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    test_tor_hs_client().await
}