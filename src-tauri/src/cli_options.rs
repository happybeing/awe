///! Command line options and usage
use sn_peers_acquisition::PeersArgs;

// pub use structopt::clap;
use clap::Parser;
use color_eyre::Result;
use core::time::Duration;

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "a web browser and website publishing app for Autonomi peer-to-peer network (demo)"
)]
pub struct Opt {
    #[command(flatten)]
    pub peers: PeersArgs,

    /// The maximum duration to wait for a connection to the network before timing out.
    #[clap(long = "timeout", global = true, value_parser = |t: &str| -> Result<Duration> { Ok(t.parse().map(Duration::from_secs)?) })]
    pub connection_timeout: Option<Duration>,
    // TODO implement remaining CLI options
}

// pub fn get_app_name() -> String {
//     String::from(???)
// }

// pub fn get_app_version() -> String {
//     String::from(structopt::clap::crate_version!())
// }
