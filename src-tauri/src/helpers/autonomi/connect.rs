// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use indicatif::ProgressBar;
use log::{error, info};

use autonomi::Client;

use crate::cli_options::Opt;

pub async fn connect_to_network() -> Result<Client> {
    let opt = Opt::parse();
    match crate::helpers::autonomi::access::network::get_peers(opt.peers).await {
        Ok(peers) => {
            let progress_bar = ProgressBar::new_spinner();
            progress_bar.enable_steady_tick(Duration::from_millis(120));
            progress_bar.set_message("Connecting to The Autonomi Network...");
            let new_style = progress_bar.style().tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈🔗");
            progress_bar.set_style(new_style);

            progress_bar.set_message("Connecting to The Autonomi Network...");

            match Client::connect(&peers).await {
                Ok(client) => {
                    info!("Connected to the Network");
                    progress_bar.finish_with_message("Connected to the Network");
                    Ok(client)
                }
                Err(e) => {
                    progress_bar.finish_with_message("Failed to connect to the network");
                    Err(eyre!("Failed to connect to the network: {e}"))
                }
            }
        }
        Err(e) => Err(eyre!("Failed to get peers: {e}")),
    }
}
