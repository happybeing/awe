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

use color_eyre::{eyre::eyre, Result};
use walkdir::WalkDir;

use crate::helpers::autonomi::access::keys::get_register_signing_key;
// use sn_cli::{ChunkManager, Estimator};
use crate::helpers::autonomi::wallet::load_wallet;

use crate::awe_const::MAIN_REPOSITORY;
use crate::awe_website_publisher::publish_website;
use crate::awe_website_versions::{self, is_compatible_network, WebsiteVersions};

use crate::awe_client;
use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    match opt.cmd {
        Some(Subcommands::Estimate { website_root }) => {
            let client = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");
            match client.file_cost(&website_root).await {
                Ok(tokens) => println!("Cost estimate: {tokens}"),
                Err(e) => println!("Unable to estimate cost: {e}"),
            }
        }
        Some(Subcommands::Publish {
            website_root,
            // update, TODO when NRS, re-instate
            website_config,
            is_new_network,
        }) => {
            // TODO move this code into a function which handles both Publish and Update
            let _ = check_website_path(&website_root);

            let mut wallet =
                load_wallet().inspect_err(|e| println!("Failed to load wallet. {}", e))?;
            let client = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");

            if !is_new_network && !is_compatible_network(&client).await {
                let message = format!(
                    "ERROR: This version of awe cannot publish to this Autonomi network\
                \nERROR: Please update awe and try again. See {MAIN_REPOSITORY}"
                )
                .clone();
                println!("{message}");
                return Err(eyre!(message));
            }

            println!("Uploading new website to network...");
            let website_address = publish_website(&website_root, website_config, &client, &wallet)
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
            let owner_secret = get_register_signing_key().inspect_err(|e| println!("{}", e))?;
            println!("got wallet... calling WebsiteVersions::new_register()");
            let mut website_versions = WebsiteVersions::new_register(
                &client,
                &mut wallet,
                &register_type,
                Some(owner_secret),
            )
            .await
            .inspect_err(|e| println!("{}", e))?;
            match website_versions
                .publish_new_version(&client, &website_address, &wallet)
                .await
            {
                Ok(version) => {
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
            website_config,
        }) => {
            let _ = check_website_path(&website_root);

            let mut wallet =
                load_wallet().inspect_err(|e| println!("Failed to load wallet. {}", e))?;
            let client = awe_client::connect_to_autonomi()
                .await
                .expect("Failed to connect to Autonomi Network");

            println!("Uploading updated website to network...");
            let owner_secret = get_register_signing_key().inspect_err(|e| println!("{}", e))?;

            let website_address =
                publish_website(&website_root, website_config, &client, &wallet).await?;

            println!("Updating versions register {}", update_xor.to_hex());
            let mut website_versions =
                match WebsiteVersions::load_register(update_xor, &client, Some(owner_secret)).await
                {
                    Ok(website_versions) => website_versions,
                    Err(e) => {
                        println!("Failed to access website versions. {}", e.root_cause());
                        return Err(e);
                    }
                };

            match website_versions
                .publish_new_version(&client, &website_address, &mut wallet)
                .await
            {
                Ok(version) => {
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
            print_audit,
            print_merkle_reg,
            entries_range,
            include_files,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_register(
                register_address,
                print_register_summary,
                print_type,
                print_size,
                print_audit,
                print_merkle_reg,
                entries_range,
                include_files,
                files_args,
            )
            .await
            {
                Ok(()) => return Ok(true),
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
            return Err(eyre!(
                "The directory specified for upload is empty. \
        Please verify the provided path."
            ));
        } else {
            return Err(eyre!(
                "The provided file path is invalid. Please verify the path."
            ));
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
