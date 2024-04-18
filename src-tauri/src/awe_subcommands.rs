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
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use clap::Parser;
use color_eyre::{eyre::bail, Result};
use walkdir::WalkDir;

use autonomi::{
    download_file, download_files, ChunkManager, Estimator, FilesUploadSummary, FilesUploader,
    UploadedFile, UPLOADED_FILES,
};

use sn_client::{Client, ClientEventsBroadcaster, FilesApi, UploadCfg, BATCH_SIZE};
use sn_protocol::storage::RetryStrategy;
use xor_name::XorName;

use crate::awe_website_publisher::publish_website;

use sn_peers_acquisition::get_peers_from_args;

use crate::awe_client;
use crate::cli_options::{Opt, Subcommands};

// Returns true if command completed
pub async fn cli_commands() -> Result<()> {
    let opt = Opt::parse();

    // Leave this here for now as a way to show if connecting is not working,
    // even though it is not used, and the handlers do this for each request.
    // TODO rationalise these steps and minimise repeats across requests.
    let client_data_dir_path = awe_client::get_client_data_dir_path()?;
    let root_dir = client_data_dir_path.clone(); // TODO make this an optional "wallet_dir" parameter

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
    let verify_store = !opt.no_verify;

    match opt.cmd {
        Some(Subcommands::Estimate {
            website_root,
            make_public,
        }) => {
            let files_api = FilesApi::build(client.clone(), root_dir.clone())?;
            let chunk_manager = ChunkManager::new(root_dir.as_path());
            Estimator::new(chunk_manager, files_api)
                .estimate_cost(website_root, make_public, root_dir.as_path())
                .await?;
            return Ok(());
        }
        Some(Subcommands::Publish {
            website_root,
            // update, TODO when NRS, re-instate
            make_public,
            website_config,
            batch_size,
            retry_strategy,
        }) => {
            let _ = check_website_path(&website_root);

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            // TODO first create the awe website versions
            publish_website(
                &website_root,
                website_config,
                make_public,
                &client,
                root_dir.as_path(),
                &upload_config,
            )
            .await;
            Ok(())
        }
        Some(Subcommands::Update {
            website_root,
            // name, TODO when NRS, re-instate
            update_xor,
            make_public,
            website_config,
            batch_size,
            retry_strategy,
        }) => {
            let _ = check_website_path(&website_root);

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            // TODO get existing awe website versions
            publish_website(
                &website_root,
                website_config,
                make_public,
                &client,
                root_dir.as_path(),
                &upload_config,
            )
            .await;
            Ok(())
        }

        // Default is not to return, but open the browser by continuing
        Some(Subcommands::Browse { website_version }) => {
            // Register protocols and open the browser
            // TODO if present, store URL parameter to be picked up by web front-end
            // TODO the URL parameter Option<String> for Browse, Fetch and no-command
            // TODO pass website_version
            println!("website_version: {website_version}");
            return Ok(());
            crate::awe_protocols::register_protocols().await;
            Ok(())
        }

        // Default is not to return, but open the browser by continuing
        None => {
            let website_version: usize = 0;
            // Register protocols and open the browser
            // TODO if present, store URL parameter to be picked up by web front-end
            // TODO the URL parameter Option<String> for Browse, Fetch and no-command
            // TODO pass website_version
            crate::awe_protocols::register_protocols().await;
            Ok(())
        }
    }
}

fn check_website_path(website_root: &PathBuf) -> Result<()> {
    let files_count = count_files_in_path_recursively(&website_root);

    if files_count == 0 {
        if website_root.is_dir() {
            bail!(
                "The directory specified for upload is empty. \
        Please verify the provided path."
            );
        } else {
            bail!("The provided file path is invalid. Please verify the path.");
        }
    }
    Ok(())
}

fn count_files_in_path_recursively(file_path: &PathBuf) -> u32 {
    let entries_iterator = WalkDir::new(file_path).into_iter().flatten();
    let mut count = 0;

    entries_iterator.for_each(|entry| {
        if entry.file_type().is_file() {
            count += 1;
        }
    });
    count
}
