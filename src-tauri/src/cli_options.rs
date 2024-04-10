/*
Copyright (c) 2024 Mark Hughes

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use clap::Parser;
use clap::Subcommand;
use color_eyre::Result;
use core::time::Duration;
use sn_peers_acquisition::PeersArgs;

use crate::subcommands::web;
///! Command line options and usage
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

    /// Available sub commands
    #[clap(subcommand)]
    pub cmd: Option<SubCmd>,

    /// The maximum duration to wait for a connection to the network before timing out.
    #[clap(long = "timeout", value_parser = |t: &str| -> Result<Duration> { Ok(t.parse().map(Duration::from_secs)?) })]
    pub connection_timeout: Option<Duration>,

    /// Prevent verification of data storage on the network.
    ///
    /// This may increase operation speed, but offers no guarantees that operations were successful.
    #[clap(global = true, long = "no-verify", short = 'x')]
    pub no_verify: bool,
    // TODO remove in favour of WebCmds subcommand
    // /// Local path of static HTML files to publish
    // #[clap(long = "publish-website")]
    // pub website_root: Option<PathBuf>,
    // TODO implement remaining CLI options:
    // TODO --wallet-path <path-to-wallet-dir>
}

// TODO add subcommands webname and fetch
#[derive(Subcommand, Debug)]
pub(super) enum SubCmd {
    /// Commands for website publishing
    #[clap(name = "web", subcommand)]
    Web(web::WebCmds),
    // /// Commands for web name management
    // TODO #[clap(name = "webname", subcommand)]
    // Webnames(webnames::RegisterCmds),
}

// pub fn get_app_name() -> String {
//     String::from(???)
// }

// pub fn get_app_version() -> String {
//     String::from(structopt::clap::crate_version!())
// }
