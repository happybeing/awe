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
use std::path::PathBuf;

use color_eyre::{eyre::bail, eyre::eyre, Result};
use walkdir::WalkDir;

use autonomi::{ChunkManager, Estimator};

use sn_client::{transfers::HotWallet, UploadCfg, WalletClient};

use crate::awe_const::MAIN_REPOSITORY;
use crate::awe_website_publisher::publish_website;
use crate::awe_website_versions::{self, is_compatible_network, WebsiteVersions};

use crate::awe_client;
use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    // Leave this here for now as a way to show if connecting is not working,
    // even though it is not used, and the handlers do this for each request.
    // TODO rationalise these steps and minimise repeats across requests.
    let client_data_dir_path = awe_client::get_client_data_dir_path()?;
    let root_dir = client_data_dir_path.clone(); // TODO ? make this an optional "wallet_dir" parameter

    // default to verifying storage
    let verify_store = !opt.no_verify;

    match opt.cmd {
        Some(Subcommands::Estimate {
            website_root,
            make_private,
        }) => {
            let files_api = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");
            let chunk_manager = ChunkManager::new(root_dir.as_path());
            Estimator::new(chunk_manager, files_api)
                .estimate_cost(website_root, !make_private, root_dir.as_path())
                .await
                .expect("Failed to estimate cost");
        }
        Some(Subcommands::Publish {
            website_root,
            // update, TODO when NRS, re-instate
            make_private,
            website_config,
            batch_size,
            retry_strategy,
            is_new_network,
        }) => {
            // TODO move this code into a function which handles both Publish and Update
            let _ = check_website_path(&website_root);

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            let files_api = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");

            if !is_new_network && !is_compatible_network(&files_api).await {
                let message = format!(
                    "ERROR: This version of awe cannot publish to this Autonomi network\
                \nERROR: Please update awe and try again. See {MAIN_REPOSITORY}"
                )
                .clone();
                println!("{message}");
                return Err(eyre!(message));
            }

            let website_address = publish_website(
                &website_root,
                website_config,
                make_private,
                &files_api.client().clone(),
                root_dir.as_path(),
                &upload_config,
            )
            .await
            .inspect_err(|e| println!("{}", e))?;

            let register_type = if is_new_network {
                website_address
            } else {
                awe_client::str_to_xor_name(
                    awe_website_versions::awv_register_type_string().as_str(),
                )?
            };

            println!("Creating versions register, please wait...");
            let mut wallet_client =
                WalletClient::new(files_api.client().clone(), HotWallet::load_from(&root_dir)?);
            let mut website_versions = WebsiteVersions::new_register(
                &files_api.client().clone(),
                &mut wallet_client,
                &register_type,
            )
            .await
            .inspect_err(|e| println!("{}", e))?;
            match website_versions
                .publish_new_version(&website_address, &mut wallet_client)
                .await
            {
                Ok((version, _storage_cost, _royalties)) => {
                    let xor_address = website_versions.versions_address().to_hex();
                    let website_root = website_root.to_str();
                    let website_root = if website_root.is_some() {
                        website_root.unwrap()
                    } else {
                        "<WEBSITE-ROOT>"
                    };
                    println!(
                        "\nWEBSITE PUBLISHED (version {version}). All versions available at XOR-URL:\nawv://{}", &xor_address
                    );
                    println!("\nNOTE:\n- To update this website, use 'awe update' as follows:\n\n   awe update --update-xor {} --website-root {}\n", &xor_address, &website_root);
                    println!("- To browse the website use 'awe awv://<XOR-ADDRESS>' as follows:\n\n   awe awv://{}\n", &xor_address);
                    println!("- For help use 'awe --help'\n");
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
            make_private,
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

            let files_api = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");

            let website_address = publish_website(
                &website_root,
                website_config,
                make_private,
                &files_api.client().clone(),
                root_dir.as_path(),
                &upload_config,
            )
            .await?;

            println!("Updating versions register...");
            let mut wallet_client =
                WalletClient::new(files_api.client().clone(), HotWallet::load_from(&root_dir)?);
            let mut website_versions =
                match WebsiteVersions::load_register(update_xor, &files_api).await {
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
                Ok((version, _storage_cost, _royalties)) => {
                    let xor_address = website_versions.versions_address().to_hex();
                    let website_root = website_root.to_str();
                    let website_root = if website_root.is_some() {
                        website_root.unwrap()
                    } else {
                        "<WEBSITE-ROOT>"
                    };
                    println!(
                        "\nWEBSITE UPDATED (version {version}). All versions available at XOR-URL:\nawv://{}", &xor_address
                    );
                    println!("\nNOTE:\n- To update this website, use 'awe update' as follows:\n\n   awe update --update-xor {} --website-root {}\n", &xor_address, &website_root);
                    println!("- To browse the website use 'awe awv://<XOR-ADDRESS>' as follows:\n\n   awe awv://{}\n", &xor_address);
                    println!("- For help use 'awe --help'\n");
                }
                Err(e) => {
                    let message = format!("Failed to update website version: {e:?}");
                    println!("{message}");
                    return Err(eyre!(message));
                }
            }
        }

        Some(Subcommands::Browse {
            url: _,
            website_version: _,
        }) => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }

        Some(Subcommands::Inspect_register {
            register_address,
            print_register_summary,
            print_type,
            print_size,
            entries_range,
            include_files,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_register(
                register_address,
                print_register_summary,
                print_type,
                print_size,
                entries_range,
                include_files,
                files_args,
            )
            .await
            {
                Ok(_) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Inspect_files {
            files_metadata_address,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_files(
                files_metadata_address,
                files_args,
            )
            .await
            {
                Ok(_) => return Ok(true),
                Err(e) => {
                    println!("{e:?}");
                    return Err(e);
                }
            }
        }

        Some(Subcommands::Download {
            awe_url: _,
            filesystem_path: _,
            entries_range: _,
            files_args: _,
        }) => {
            println!("TODO: implement subcommand download");
        }

        // Default is not to return, but open the browser by continuing
        None {} => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
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
