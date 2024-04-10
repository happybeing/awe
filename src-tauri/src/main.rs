/*
*   Copyright (c) 2024 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU General Public License for more details.

*   You should have received a copy of the GNU General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod awe_client;
mod awe_protocols;
mod awe_websites;
mod cli_options;
mod subcommands;

use clap::Parser;
use color_eyre::Result;
use sn_client::{
    Client, ClientEvent, ClientEventsBroadcaster, ClientEventsReceiver, FilesApi, FilesDownload,
};
use sn_peers_acquisition::get_peers_from_args;

use cli_options::{Opt, SubCmd};
use subcommands::web::web_cmds;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::parse();

    // Leave this here for now as a way to show if connecting is not working,
    // even though it is not used, and the handlers do this for each request.
    // TODO rationalise these steps and minimise repeats across requests.
    let client_data_dir_path = awe_client::get_client_data_dir_path()?;

    println!("Initialising Autonomi client...");
    let secret_key = awe_client::get_client_secret_key(&client_data_dir_path)?;

    let bootstrap_peers = get_peers_from_args(opt.peers).await?;

    println!(
        "Connecting to the network with {} peers",
        bootstrap_peers.len(),
    );

    let bootstrap_peers = if bootstrap_peers.is_empty() {
        // empty vec is returned if `local-discovery` flag is provided
        None
    } else {
        Some(bootstrap_peers)
    };

    // get the broadcaster as we want to have our own progress bar.
    let broadcaster = ClientEventsBroadcaster::default();
    let progress_bar_handler = awe_client::spawn_connection_progress_bar(broadcaster.subscribe());

    let result = Client::new(
        secret_key,
        bootstrap_peers,
        opt.connection_timeout,
        Some(broadcaster),
    )
    .await;

    // await on the progress bar to complete before handling the client result. If client errors out, we would
    // want to make the progress bar clean up gracefully.
    progress_bar_handler.await?;
    let client = result?;

    // default to verifying storage
    let should_verify_store = !opt.no_verify;

    match opt.cmd {
        Some(SubCmd::Web(cmds)) => {
            web_cmds(cmds, &client, &client_data_dir_path, should_verify_store).await?;
            return Ok(());
        }
        // TODO Webname commands
        // SubCmd::WebnameCmds(cmds) => {
        //     webname_cmds(cmds, &client, &client_data_dir_path, should_verify_store).await?
        // }

        // Default is not to return, but open the browser by continuing
        None => {}
    };

    // Registers protocols and open the browser
    crate::awe_protocols::register_protocols().await;
    Ok(())
}
