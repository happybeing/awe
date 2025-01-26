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
use chrono::offset::Utc;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Result};
use xor_name::XorName;

use ant_protocol::storage::{Pointer, PointerAddress};

use dweb::autonomi::access::network::NetworkPeers;
use dweb::trove::directory_tree::DirectoryTree;

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'inspect-history' subcommand
pub async fn handle_inspect_history(
    peers: NetworkPeers,
    pointer_address: PointerAddress,
    print_summary: bool,
    print_type: bool,
    print_size: bool,
    entries_range: Option<EntriesRange>,
    include_files: bool,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    let pointer = match client.client.pointer_get(pointer_address).await {
        Ok(pointer) => pointer,
        Err(e) => {
            let message = format!("Failed to get pointer from network - {e}");
            println!("{message}");
            return Err(eyre!(message));
        }
    };

    let count = pointer.counter();
    if print_summary {
        do_print_summary(&pointer, &pointer_address)?;
    } else {
        if print_type {
            // if size > 0 {
            //     do_print_type(Some(&entries_vec[0]))?;
            // } else {
            //     do_print_type(None)?;
            // }
        }

        if print_size {
            // do_print_size(size)?;
        }
    }

    // if let Some(entries_range) = entries_range {
    //     do_print_entries(
    //         &client,
    //         &entries_range,
    //         entries_vec,
    //         include_files,
    //         &files_args,
    //     )
    //     .await?;
    // };

    Ok(())
}

fn do_print_summary(pointer: &Pointer, pointer_address: &PointerAddress) -> Result<()> {
    println!("pointer     : {}", pointer_address.to_hex());
    // println!("owner       : {:?}", pointer.owner());
    // println!("permissions : {:?}", pointer.permissions());
    println!("count       : {:?}", pointer.counter());

    // if entries_vec.len() > 0 {
    //     do_print_type(Some(&entries_vec[0]))?;
    // } else {
    //     do_print_type(None)?;
    // }
    // do_print_size(size)?;
    Ok(())
}

fn do_print_type(history_type: Option<XorName>) -> Result<()> {
    if history_type.is_some() {
        println!("history type: {:64x}", history_type.unwrap());
    } else {
        println!("history type: not set");
    }
    Ok(())
}

fn do_print_size(size: usize) -> Result<()> {
    println!("size        : {size}");
    Ok(())
}

// async fn do_print_entries(
//     client: &AutonomiClient,
//     entries_range: &EntriesRange,
//     entries_vec: Vec<Entry>,
//     include_files: bool,
//     files_args: &FilesArgs,
// ) -> Result<()> {
//     let size = entries_vec.len();
//     if size == 0 {
//         return Ok(());
//     }

//     let first = if entries_range.start.is_some() {
//         entries_range.start.unwrap()
//     } else {
//         0
//     };

//     let last = if entries_range.end.is_some() {
//         entries_range.end.unwrap()
//     } else {
//         size - 1
//     };

//     if last > size - 1 {
//         return Err(eyre!(
//             "range exceeds maximum history entry which is {}",
//             size - 1
//         ));
//     }

//     // As entries_vec[] is in reverse order we adjust the start and end and count backwards
//     println!("entries {first} to {last}:");
//     for index in first..=last {
//         let xor_name = xorname_from_entry(&entries_vec[index]);
//         if include_files {
//             println!("entry {index} - fetching metadata at {xor_name:64x}");
//             match DirectoryTree::directory_tree_download(client, xor_name).await {
//                 Ok(metadata) => {
//                     let _ = do_print_files(&metadata, &files_args);
//                 }
//                 Err(e) => {
//                     println!("Failed to get website metadata from network");
//                     return Err(eyre!(e));
//                 }
//             };
//         } else {
//             println!("{xor_name:64x}");
//         }
//     }

//     Ok(())
// }

fn do_print_files(metadata: &DirectoryTree, files_args: &FilesArgs) -> Result<()> {
    let metadata_stats = if files_args.print_metadata_summary
        || files_args.print_counts
        || files_args.print_total_bytes
    {
        metadata_stats(metadata)?
    } else {
        (0 as usize, 0 as u64)
    };

    if files_args.print_metadata_summary {
        let _ = do_print_metadata_summary(metadata, metadata_stats);
    } else {
        if files_args.print_counts {
            let _ = do_print_counts(metadata, metadata_stats.0);
        }

        if files_args.print_total_bytes {
            let _ = do_print_total_bytes(metadata_stats.1);
        }
    }

    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in metadata.path_map.paths_to_files_map.iter() {
            for (file_name, xor_name, modified, size, json_metadata) in path_map.iter() {
                if files_args.print_all_details {
                    let date_time = DateTime::<Utc>::from(*modified);
                    let modified_str = date_time.format("%Y-%m-%d %H:%M:%S").to_string();
                    println!(
                        "{xor_name:64x} {modified_str} \"{path_string}{file_name}\" {size} bytes and JSON: \"{json_metadata}\"",
                    );
                } else {
                    println!("{xor_name:64x} \"{path_string}{file_name}\"");
                }
            }
        }
    }

    Ok(())
}

fn metadata_stats(metadata: &DirectoryTree) -> Result<(usize, u64)> {
    let mut files_count: usize = 0;
    let mut total_bytes: u64 = 0;

    for (_, path_map) in metadata.path_map.paths_to_files_map.iter() {
        files_count = files_count + path_map.len();

        for file_metadata in path_map {
            total_bytes = total_bytes + file_metadata.3
        }
    }

    Ok((files_count, total_bytes))
}

fn do_print_metadata_summary(metadata: &DirectoryTree, metadata_stats: (usize, u64)) -> Result<()> {
    println!("published  : {}", metadata.date_published);
    let _ = do_print_counts(metadata, metadata_stats.0);
    let _ = do_print_total_bytes(metadata_stats.1);
    Ok(())
}

fn do_print_counts(metadata: &DirectoryTree, count_files: usize) -> Result<()> {
    println!(
        "directories: {}",
        metadata.path_map.paths_to_files_map.len()
    );
    println!("files      : {count_files}");
    Ok(())
}

fn do_print_total_bytes(total_bytes: u64) -> Result<()> {
    println!("total bytes: {total_bytes}");
    Ok(())
}

/// Implement 'inspect-files' subcommand
///
/// Accepts a metadata address
pub async fn handle_inspect_files(
    peers: NetworkPeers,
    directory_address: XorName,
    files_args: FilesArgs,
) -> Result<()> {
    let client = dweb::client::AutonomiClient::initialise_and_connect(peers)
        .await
        .expect("Failed to connect to Autonomi Network");

    println!("fetching directory at {directory_address:64x}");
    match DirectoryTree::directory_tree_download(&client, directory_address).await {
        Ok(metadata) => {
            let _ = do_print_files(&metadata, &files_args);
        }
        Err(e) => {
            println!("Failed to get directory from network");
            return Err(eyre!(e).into());
        }
    };
    Ok(())
}
