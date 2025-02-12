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

use color_eyre::Result;

use dweb::storage::{publish_or_update_files, report_content_published_or_updated};

use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    let peers = dweb::autonomi::access::network::get_peers(opt.peers);

    match opt.cmd {
        Some(Subcommands::Estimate { files_root }) => {
            let client = dweb::client::AutonomiClient::initialise_and_connect(peers.await?)
                .await
                .expect("Failed to connect to Autonomi Network");
            match client.client.file_cost(&files_root).await {
                Ok(tokens) => println!("Cost estimate: {tokens}"),
                Err(e) => println!("Unable to estimate cost: {e}"),
            }
        }
        Some(Subcommands::Publish_new {
            files_root,
            name,
            website_config,
            is_new_network: _,
        }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let client = dweb::client::AutonomiClient::initialise_and_connect(peers.await?)
                .await
                .expect("Failed to connect to Autonomi Network");

            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                website_config,
                true,
            )
            .await
            {
                Ok(history_address) => history_address,
                Err(e) => {
                    println!("Failed to publish files: {e}");
                    return Err(e);
                }
            };

            report_content_published_or_updated(
                &history_address,
                &name,
                version,
                cost,
                &files_root,
                true,
                true,
                true,
            );
        }
        Some(Subcommands::Publish_update {
            files_root,
            name,
            website_config,
        }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let client = dweb::client::AutonomiClient::initialise_and_connect(peers.await?)
                .await
                .expect("Failed to connect to Autonomi Network");

            let (cost, name, history_address, version) = publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                website_config,
                false,
            )
            .await?;

            report_content_published_or_updated(
                &history_address,
                &name,
                version,
                cost,
                &files_root,
                true,
                false,
                true,
            );
        }

        Some(Subcommands::Browse {
            url: _,
            history_version: _,
        }) => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }

        Some(Subcommands::Inspect_history {
            history_address,
            print_history_summary,
            print_type,
            print_size,
            entries_range,
            include_files,
            files_args,
        }) => {
            match crate::commands::cmd_inspect::handle_inspect_history(
                peers.await?,
                history_address,
                print_history_summary,
                print_type,
                print_size,
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
                peers.await?,
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
            println!("TODO: implement subcommand 'download'");
        }

        Some(Subcommands::Serve { host: _, port: _ }) => {
            println!("TODO: implement subcommand 'serve'"); // THIS IS REALLY FOR DWEB-CLI
        }

        // Default is not to return, but open the browser by continuing
        None {} => {
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
}
