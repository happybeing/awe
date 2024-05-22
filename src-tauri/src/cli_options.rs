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
use sn_registers::RegisterAddress;
use std::path::PathBuf;

use sn_peers_acquisition::PeersArgs;
use sn_protocol::storage::RetryStrategy;

use crate::awe_client::{str_to_register_address};

// TODO add example to each CLI subcommand

///! Command line options and usage
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "a web browser and website publishing app for Autonomi peer-to-peer network (demo)"
)]
pub struct Opt {
    /// Optional awe URL to browse.
    ///
    /// Use amx://<XOR-ADDRESS> to browse a website (use --website-version to specify a version).
    ///
    /// Use xor://<XOR-ADDRESS> to load or fetch to a file rather than a website.
    // TODO mention awx://name
    // TODO implement fetch subcommand
    pub url: Option<String>,

    /// Browse the specified website version
    #[clap(long, short = 'w', value_parser = greater_than_0)]
    pub website_version: Option<u64>,

    #[command(flatten)]
    pub peers: PeersArgs,

    /// Available sub commands
    #[command(subcommand)]
    pub cmd: Option<Subcommands>,

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

fn greater_than_0(s: &str) -> Result<u64, String> {
    match s.parse::<u64>() {
        Err(e) => Err(e.to_string()),
        Ok(value) => {
            if value >= 1 {
                Ok(value)
            } else {
                Err(String::from("Number must be greater than zero"))
            }
        }
    }
}

// TODO add subcommands webname and fetch
#[derive(Subcommand, Debug)]
pub enum Subcommands {
    /// Open the browser (this is the default if no command is given).
    Browse {
        /// Optional awe URL to browse.
        ///
        /// Use amx://<XOR-ADDRESS> to browse a website (use --website-version to specify a version).
        ///
        /// Use xor://<XOR-ADDRESS> to load or fetch to a file rather than a website.
        // TODO mention awx://name
        // TODO implement fetch subcommand
        url: Option<String>,

        /// Browse the specified website version
        #[clap(long, short = 'w', value_parser = greater_than_0)]
        website_version: Option<u64>,
    },

    // TODO add an example or two to each command section
    /// Estimate the cost of publishing or updating a website
    Estimate {
        /// The root directory containing the website content to be published
        #[clap(long = "website-root", value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        /// Should the website content be made accessible to all. (This is irreversible.)
        ///
        /// Note that without access to the website metadata even public data won't be
        /// discoverable. Access will only be possible with the address of the metadata
        /// or of the individual uploaded pages and resources contained in the metadata.
        #[clap(long, name = "make-public", default_value = "true", short = 'p')]
        make_public: bool,
    },

    /// Publish a new website
    ///
    /// Uploads a tree of website files to Autonomi and pays using the default wallet
    ///
    /// If successful, prints the xor address of the website, accessible
    /// using Awe Browser using a URL like 'awx://<XOR-ADDRESS>'.
    Publish {
        /// The root directory containing the website content to be published
        #[clap(long = "website-root", value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        // TODO when NRS, re-instate the following (and 'conflicts_with = "update"' above)
        // /// Update the website at given awe NRS name
        // #[clap(
        //     long,
        //     short = 'n',
        //     conflicts_with = "update_xor"
        // )]
        // name: String,
        /// Optional website configuration such as default index file(s), redirects etc.
        #[clap(long = "website-config", short = 'c', value_name = "JSON-FILE")]
        website_config: Option<PathBuf>,
        /// The batch_size to split chunks into parallel handling batches
        /// during payment and upload processing.
        #[clap(long, default_value_t = sn_client::BATCH_SIZE, short='b')]
        batch_size: usize,
        /// Should the website content be made accessible to all. (This is irreversible.)
        ///
        /// Note that without access to the website metadata even public data won't be
        /// discoverable. Access will only be possible with the address of the metadata
        /// or of the individual uploaded pages and resources contained in the metadata.
        #[clap(long, name = "make-public", default_value = "true", short = 'p')]
        make_public: bool,
        /// Set the strategy to use on chunk upload failure. Does not modify the spend failure retry attempts yet.
        ///
        /// Choose a retry strategy based on effort level, from 'quick' (least effort), through 'balanced',
        /// to 'persistent' (most effort).
        #[clap(long, default_value_t = RetryStrategy::Balanced, short = 'r', help = "Sets the retry strategy on upload failure. Options: 'quick' for minimal effort, 'balanced' for moderate effort, or 'persistent' for maximum effort.")]
        retry_strategy: RetryStrategy,
    },

    /// Update an existing website while preserving old versions on Autonomi
    ///
    /// Uploads changes in the website content directory and makes this the
    /// default version. Pays using the default wallet.
    ///
    /// If successful upload prints the xor address of the website, accessible
    /// using Awe Browser using a URL like 'awx://<XOR-ADDRESS>'.
    Update {
        /// The root directory containing the new website content to be uploaded
        #[clap(long = "website-root", value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        /// Xor address of website to be updated
        #[clap(long, value_parser = str_to_register_address)]
        update_xor: RegisterAddress,
        // TODO when NRS, re-instate the following (and 'conflicts_with = "update"' above)
        // /// Update the website at given awe NRS name
        // #[clap(
        //     long,
        //     short = 'u',
        //     conflicts_with = "new",
        //     conflicts_with = "estimate_cost",
        //     conflicts_with = "update_xor"
        // )]
        // update: String,
        /// Optional website configuration such as default index file(s), redirects etc.
        #[clap(long = "website-config", short = 'c', value_name = "JSON-FILE")]
        website_config: Option<PathBuf>,
        /// The batch_size to split chunks into parallel handling batches
        /// during payment and upload processing.
        #[clap(long, default_value_t = sn_client::BATCH_SIZE, short='b')]
        batch_size: usize,
        /// Should the website content be made accessible to all. (This is irreversible.)
        ///
        /// Note that without access to the website metadata even public data won't be
        /// discoverable. Access will only be possible with the address of the metadata
        /// or of the individual uploaded pages and resources contained in the metadata.
        #[clap(long, name = "make-public", default_value = "true", short = 'p')]
        make_public: bool,
        /// Set the strategy to use on chunk upload failure. Does not modify the spend failure retry attempts yet.
        ///
        /// Choose a retry strategy based on effort level, from 'quick' (least effort), through 'balanced',
        /// to 'persistent' (most effort).
        #[clap(long, default_value_t = RetryStrategy::Balanced, short = 'r', help = "Sets the retry strategy on upload failure. Options: 'quick' for minimal effort, 'balanced' for moderate effort, or 'persistent' for maximum effort.")]
        retry_strategy: RetryStrategy,
    },
}

// pub fn get_app_name() -> String {
//     String::from(???)
// }

// pub fn get_app_version() -> String {
//     String::from(structopt::clap::crate_version!())
// }
