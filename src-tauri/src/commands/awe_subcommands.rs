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

use color_eyre::{Report, Result};

use autonomi::AttoTokens;

use dweb::client::{ApiControl, DwebClient};
use dweb::history::HistoryAddress;
use dweb::storage::{publish_or_update_files, report_content_published_or_updated};
use dweb::token::{show_spend_return_value, Spends};

use crate::cli_options::{Opt, Subcommands};

// Returns true if command complete, false to start the browser
pub async fn cli_commands(opt: Opt) -> Result<bool> {
    let api_control = ApiControl {
        tries: opt.retry_api,
        upload_file_by_file: opt.upload_file_by_file,
        ignore_pointers: opt.ignore_pointers,
        max_fee_per_gas: opt.max_fee_per_gas,
        ..Default::default()
    };

    match opt.cmd {
        Some(Subcommands::Estimate { files_root }) => {
            let (client, _is_local_network) =
                connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            match client.client.file_cost(&files_root).await {
                Ok(tokens) => println!("Cost estimate: {tokens}"),
                Err(e) => println!("Unable to estimate cost: {e}"),
            }
        }
        Some(Subcommands::Publish_new {
            files_root,
            name,
            is_new_network: _,
        }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            let spends = Spends::new(&client, Some(&"Publish new cost: ")).await?;

            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                None,
                true,
            )
            .await
            {
                Ok(result) => {
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u32)>(
                        &spends, result,
                    )
                    .await
                }
                Err(e) => {
                    println!("Failed to publish files: {e}");
                    return show_spend_return_value::<Result<bool, Report>>(&spends, Err(e)).await;
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
        Some(Subcommands::Publish_update { files_root, name }) => {
            let app_secret_key = dweb::helpers::get_app_secret_key()?;
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            let spends = Spends::new(&client, Some(&"Publish new cost: ")).await?;
            let (cost, name, history_address, version) = match publish_or_update_files(
                &client,
                &files_root,
                app_secret_key,
                name,
                None,
                false,
            )
            .await
            {
                Ok(result) => {
                    show_spend_return_value::<(AttoTokens, String, HistoryAddress, u32)>(
                        &spends, result,
                    )
                    .await
                }
                Err(e) => {
                    println!("Failed to publish files: {e}");
                    return show_spend_return_value::<Result<bool, Report>>(&spends, Err(e)).await;
                }
            };

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

        Some(Subcommands::Inspect_history {
            history_address,
            print_history_full,
            entries_range,
            shorten_hex_strings,
            include_files,
            graph_keys,
            files_args,
        }) => {
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            match crate::commands::cmd_inspect::handle_inspect_history(
                client,
                history_address,
                print_history_full,
                entries_range,
                include_files,
                graph_keys,
                shorten_hex_strings,
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

        Some(Subcommands::Inspect_graphentry {
            graph_entry_address,
            print_full,
            shorten_hex_strings,
        }) => {
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            match crate::commands::cmd_inspect::handle_inspect_graphentry(
                client,
                graph_entry_address,
                print_full,
                shorten_hex_strings,
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

        Some(Subcommands::Inspect_pointer { pointer_address }) => {
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            match crate::commands::cmd_inspect::handle_inspect_pointer(client, pointer_address)
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
            archive_address,
            files_args,
        }) => {
            let (client, _) = connect_and_announce(opt.local, opt.alpha, api_control, true).await;
            match crate::commands::cmd_inspect::handle_inspect_files(
                client,
                archive_address,
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

        // Default is not to return, but open the browser by continuing
        None {} => {
            println!("No command provided, try 'dweb --help'");
            return Ok(false); // Command not yet complete, is the signal to start browser
        }
    }
    Ok(true)
}

pub async fn connect_and_announce(
    local_network: bool,
    alpha_network: bool,
    api_control: ApiControl,
    announce: bool,
) -> (DwebClient, bool) {
    let client =
        dweb::client::DwebClient::initialise_and_connect(local_network, alpha_network, api_control)
            .await
            .expect("Failed to connect to Autonomi Network");

    if announce {
        if local_network {
            println!("-> local network: {}", client.network);
        } else if alpha_network {
            println!("-> alpha network {}", client.network);
        } else {
            println!("-> public network {}", client.network);
        };
    };

    (client, local_network)
}
