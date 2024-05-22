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
use std::path::PathBuf;

use clap::Parser;
use color_eyre::{eyre::bail, eyre::eyre, Result};
use walkdir::WalkDir;

use autonomi::{ChunkManager, Estimator};

use sn_client::{
    transfers::HotWallet, Client, ClientEventsBroadcaster, FilesApi, UploadCfg, WalletClient,
};

use crate::awe_website_publisher::publish_website;
use crate::awe_website_versions::WebsiteVersions;

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
    let website_version = opt.website_version;

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
            // TODO move this code into a function which handles both Publish and Update
            let _ = check_website_path(&website_root);

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            let website_address = publish_website(
                &website_root,
                website_config,
                make_public,
                &client,
                root_dir.as_path(),
                &upload_config,
            )
            .await
            .inspect_err(|e| println!("{}", e))?;

            println!("Creating versions register, please wait...");
            let mut wallet_client =
                WalletClient::new(client.clone(), HotWallet::load_from(&root_dir)?);
            let mut website_versions = WebsiteVersions::new_register(&client, &mut wallet_client)
                .await
                .inspect_err(|e| println!("{}", e))?;
            match website_versions
                .publish_new_version(&website_address, &mut wallet_client)
                .await
            {
                Ok((version, storage_cost, royalties)) => {
                    println!(
                        "\nWEBSITE PUBLISHED (version {version}). All versions available at XOR-URL:\nawx://{}",
                        website_versions.versions_address().to_hex()
                    );
                    println!("\nNOTE:\n- to update this website, use 'awe update --update-xor' followed by the above xor address, and add '--website-root' followed by the path to the updated website content.");
                    println!("- you can browse this website using 'awe awx://<XOR-ADDRESS>' where <XOR-ADDRESS> is the above xor address.");
                }
                Err(e) => {
                    println!("Failed to publish new website version: {}", e.root_cause());
                    return Err(e);
                }
            }
        }
        Some(Subcommands::Update {
            website_root,
            // name: String, // TODO when NRS, re-instate name
            update_xor,
            make_public,
            website_config,
            batch_size,
            retry_strategy,
        }) => {
            // TODO move this code into a function which handles both Publish and Update
            let _ = check_website_path(&website_root);

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            let website_address = publish_website(
                &website_root,
                website_config,
                make_public,
                &client,
                root_dir.as_path(),
                &upload_config,
            )
            .await?;

            println!("Updating versions register...");
            let mut wallet_client =
                WalletClient::new(client.clone(), HotWallet::load_from(&root_dir)?);
            let files_api = FilesApi::build(client.clone(), root_dir.clone())?;
            let mut website_versions =
                match WebsiteVersions::load_register(update_xor, &client, &files_api).await {
                    Ok(website_versions) => website_versions,
                    Err(e) => {
                        println!("Failed to access website versions. {}", e.root_cause());
                        return Err(e);
                    }
                };

            match website_versions
                .publish_new_version(&website_address, &mut wallet_client)
                .await
            {
                Ok((version, storage_cost, royalties)) => {
                    println!(
                        "\nWEBSITE PUBLISHED (version {version}). All versions available at XOR-URL:\nawx://{}",
                        website_versions.versions_address().to_hex()
                    );
                    println!("\nNOTE:\n- to update this website, use 'awe update --update-xor' followed by the above xor address, and add '--website-root' followed by the path to the updated website content.");
                    println!("- you can browse this website using 'awe awx://<XOR-ADDRESS>' where <XOR-ADDRESS> is the above xor address.");
                }
                Err(e) => {
                    let message = format!("Failed to update website version: {e:?}");
                    println!("{message}");
                    return Err(eyre!(message));
                }
            }
        }

        // Default is not to return, but open the browser by continuing
        Some(Subcommands::Browse {
            url,
            website_version,
        }) => {
            // Register protocols and open the browser
            crate::awe_protocols::register_protocols(url, website_version).await;
        }

        // Default is not to return, but open the browser by continuing
        None {} => {
            // Register protocols and open the browser
            crate::awe_protocols::register_protocols(opt.url, opt.website_version).await;
        }
    }
    Ok(())
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
