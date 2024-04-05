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

mod autonomi_client;
mod autonomi_protocols;
mod cli_options;

use clap::Parser;
use color_eyre::Result;
use sn_client::FilesApi;
use sn_peers_acquisition::get_peers_from_args;

use cli_options::Opt;

#[tokio::main]
async fn main() -> Result<()> {
    // TODO make some of the objects here static: app_config / client / files_api

    // TODO review technique used to call async within protocol handlers (discouraged here: https://stackoverflow.com/questions/66035290/how-do-i-await-a-future-inside-a-non-async-method-which-was-called-from-an-async)
    // TODO Tauri is supposed to support calling async inside protocol handler, but I didn't find a recommended method
    // TODO ASKED Tauri Discord: https://discord.com/channels/616186924390023171/1224053280565497896
    // TODO ASKED Stack Overflow:https://stackoverflow.com/questions/78255320/what-is-the-recommended-technique-for-calling-an-async-function-within-a-tauri-p
    // TODO Notes: Discord solution re Tauri 2.0 - thought I was using v2.0?
    // TODO Notes: Discord solution suggests I can use spawn after all, worth trying as this blocks for ages
    tauri::Builder::default()
        .register_uri_scheme_protocol("axor", move |_app, req| {
            let handle = tokio::runtime::Handle::current();
            let _guard = handle.enter();
            // initialise safe network connection and files api
            let client = futures::executor::block_on(async move {
                let opt = Opt::parse();
                let peers = get_peers_from_args(opt.peers).await?;
                let timeout = opt.connection_timeout;
                autonomi_client::connect_to_autonomi(peers, timeout).await
            })
            .expect("Failed to connect to Autonomi Network");
            let wallet_dir = autonomi_client::get_client_data_dir_path()
                .expect("Failed to get client data dir path");
            let files_api = FilesApi::build(client.clone(), wallet_dir)
                .expect("Failed to instantiate FilesApi");
            futures::executor::block_on(async move {
                crate::autonomi_protocols::handle_protocol_axor(&client, &req, &files_api.clone())
                    .await
            })
        })
        .register_uri_scheme_protocol("axweb", move |_app, req| {
            let handle = tokio::runtime::Handle::current();
            let _guard = handle.enter();
            let client = futures::executor::block_on(async move {
                let opt = Opt::parse();
                let peers = get_peers_from_args(opt.peers).await?;
                let timeout = opt.connection_timeout;
                autonomi_client::connect_to_autonomi(peers, timeout).await
            })
            .expect("Failed to connect to Autonomi Network");
            let wallet_dir = autonomi_client::get_client_data_dir_path()
                .expect("Failed to get client data dir path");
            let files_api = FilesApi::build(client.clone(), wallet_dir)
                .expect("Failed to instantiate FilesApi");
            futures::executor::block_on(async move {
                crate::autonomi_protocols::handle_protocol_axweb(&client, &req, &files_api.clone())
                    .await
            })
        })
        // This does nothing:
        // TODO try using CSP to block other protocols. Review this: https://stackoverflow.com/questions/77806138/what-is-the-correct-way-to-configure-csp-in-tauri-when-using-css-in-js-libraries
        .register_uri_scheme_protocol("http", |_app, req| {
            println!("http-scheme: {req:?}");
            tauri::http::ResponseBuilder::new().body(Vec::new())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
