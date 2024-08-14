/*
Copyright (c) 2024-2025 Mark Hughes

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use color_eyre::{eyre::eyre, Result};
use core::time::Duration;
use sn_registers::RegisterAddress;
use std::path::PathBuf;

use sn_peers_acquisition::PeersArgs;
use sn_protocol::storage::RetryStrategy;

use crate::awe_client::str_to_register_address;

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
    /// Use awm://<XOR-ADDRESS> to browse a website (use --website-version to specify a version).
    ///
    /// Use awx://<XOR-ADDRESS> to load or fetch to a file rather than a website.
    // TODO mention awv://name
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
        /// Use awm://<XOR-ADDRESS> to browse a website (use --website-version to specify a version).
        ///
        /// Use awx://<XOR-ADDRESS> to load or fetch to a file rather than a website.
        // TODO mention awv://name
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
    /// using Awe Browser using a URL like 'awv://<XOR-ADDRESS>'.
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
        ///
        /// Disable the AWV check when publishing a new website to allow for init of a new Autonomi network (during beta)
        #[clap(long, name = "is-new-network", hide = true, default_value = "false")]
        is_new_network: bool,
    },

    /// Update an existing website while preserving old versions on Autonomi
    ///
    /// Uploads changes in the website content directory and makes this the
    /// default version. Pays using the default wallet.
    ///
    /// If successful upload prints the xor address of the website, accessible
    /// using Awe Browser using a URL like 'awv://REGISTER-ADDRESS'.
    Update {
        /// The root directory containing the new website content to be uploaded
        #[clap(long = "website-root", value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        /// The address of a register referencing each version of the website. Can begin with "awv://"
        #[clap(long, name = "REGISTER-ADDRESS", value_parser = str_to_register_address)]
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

    /// Download a file or directory
    #[clap(hide = true)] // TODO hide until implemented
    Download {
        /// An awe compatible URL. Must be an xor address prefixed with 'awx://', 'awm://' or 'awv://' respectively
        /// to reference a file, some files metadata or a register with entries of files metadata.
        ///
        /// For a metadata address you may specify the path of a specific file or directory to be downloaded
        /// by including this at the end of the AWE-URL. This defaults to the metadata root (or '/').
        ///
        /// For a register, you must provide the RANGE of entries to be processed.
        ///
        /// If you do not specify a DOWNLOAD-PATH the content downloaded will be printed
        /// on the terminal (via stdout).
        // TODO implement a parser so I can validate here any combo of protocols (but keep as String here)
        #[clap(value_name = "AWE-URL")]
        awe_url: String,

        /// A file or directory path where downloaded data is to be stored. This must not exist.
        /// If downloading more than a single file, DOWNLOAD-PATH must end with a file separator, and
        /// a directory will be created to hold the downloaded files and any subdirectories.
        #[clap(value_name = "DOWNLOAD-PATH")]
        /// TODO: PathBuf?
        filesystem_path: Option<String>,

        /// If AWE-URL is a register (i.e. starts with 'awv://') you must specify the entry or
        /// entries you with to download with this option. The download will be applied for each
        /// entry in RANGE, which can be an integer (for a single entry), or an integer followed
        /// by ':' or two integers separated by ':'. The first entry is position 0 and the last is
        /// register 'size minus 1'. When more than one entry is downloaded, each will be saved in
        /// a separate subdirectory of the <DOWNLOAD-PATH>, named with a 'v' followed by the index
        /// of the entry, such as 'v3', 'v4' etc.
        #[clap(long = "entries", short = 'e', value_name = "RANGE", value_parser = str_to_entries_range)]
        entries_range: Option<EntriesRange>,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Print information about data structures stored on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_register {
        /// The address of an Autonomi register. Can be prefixed with awv://
        #[clap(name = "REGISTER-ADDRESS", value_parser = str_to_register_address)]
        register_address: RegisterAddress,

        /// Print a summary of the register including type (the value of entry 0) and number of entries
        #[clap(long, name = "register-summary", default_value = "false")]
        print_register_summary: bool,

        /// Print the type of metadata recorded in the register (the value of entry 0)
        #[clap(long, name = "type", default_value = "false")]
        print_type: bool,

        /// Print the number of entries
        #[clap(long, name = "size", default_value = "false")]
        print_size: bool,

        /// Print information about each entry in RANGE, which can be
        /// an integer (for a single entry), or an integer followed by ':' or
        /// two integers separated by ':'. The first entry is position 0
        /// and the last is register 'size minus 1'
        #[clap(long = "entries", short = 'e', value_name = "RANGE", value_parser = str_to_entries_range )]
        entries_range: Option<EntriesRange>,

        /// For each entry in RANGE print information about files stored on
        /// the network, as recorded by the metadata pointed to by the entry. Enables
        /// the following 'print' options for files metadata entries in RANGE
        #[clap(
            long,
            name = "include-files",
            default_value = "false",
            requires = "entries_range"
        )]
        include_files: bool,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Print information about files from stored metadata
    #[allow(non_camel_case_types)]
    #[clap(hide = true)] // TODO hide until implemented
    Inspect_files {
        /// The Autonomi network address of some awe metadata. Can be prefixed with awm://
        #[clap(value_name = "FILES-METADATA-ADDRESS")]
        files_metadata_address: String,

        #[command(flatten)]
        files_args: FilesArgs,
    },
}

#[derive(Args, Debug)]
pub struct FilesArgs {
    /// Print summary information about files based on files metadata
    #[clap(long, name = "files-summary", default_value = "false")]
    pub print_files_summary: bool,

    /// Print the number of directories
    #[clap(long, name = "count-directories", default_value = "false")]
    pub print_count_directories: bool,

    /// Print the number of files
    #[clap(long, name = "count-files", default_value = "false")]
    pub print_count_files: bool,

    /// TODO Print the total number of bytes for all files
    #[clap(long, hide = true, name = "total-bytes", default_value = "false")]
    pub print_total_bytes: bool,

    /// Print the path of each file
    #[clap(long, name = "paths", default_value = "false")]
    pub print_paths: bool,

    /// TODO Print detailed information about each file including path and size in bytes
    #[clap(long, hide = true, name = "details", default_value = "false")]
    pub print_details: bool,
}

use regex::Regex;
#[derive(Clone, Debug)]
pub struct EntriesRange {
    pub start: Option<usize>,
    pub end: Option<usize>,
}

fn str_to_entries_range(s: &str) -> Result<EntriesRange> {
    lazy_static::lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d*)(:?)(\d*)$").unwrap();
    }

    let captures = match RE.captures(s) {
        Some(captures) => captures,
        None => return Err(eyre!("invalid range")),
    };

    let start = if !captures[1].is_empty() {
        match captures[1].parse::<usize>() {
            Ok(n) => Some(n),
            Err(_) => return Err(eyre!("invalid start value")),
        }
    } else {
        None
    };

    let end = if start.is_some() && captures[2].is_empty() {
        start
    } else {
        if !captures[3].is_empty() {
            match captures[3].parse::<usize>() {
                Ok(n) => Some(n),
                Err(_) => return Err(eyre!("invalid end value")),
            }
        } else {
            None
        }
    };

    if let (Some(start), Some(end)) = (start, end) {
        if end < start {
            return Err(eyre!("end cannot be less than start"));
        }
    }

    Ok(EntriesRange { start, end })
}

// pub fn get_app_name() -> String {
//     String::from(???)
// }

// pub fn get_app_version() -> String {
//     String::from(structopt::clap::crate_version!())
// }
