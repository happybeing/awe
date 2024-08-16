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

use crate::awe_client;
use crate::awe_website_metadata::{get_website_metadata_from_network, WebsiteMetadata};
use crate::cli_options::{EntriesRange, FilesArgs};
use crate::commands::helpers;
use chrono::offset::Utc;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Result};
use sn_client::FilesApi;
use sn_registers::{Entry, RegisterAddress};
use xor_name::XorName;

/// Implement 'inspect-register' subcommand
///
/// Treats a register as a vector of nodes with one Entry per element, so branches are treated as a single node
///
/// TODO extend treatment to handle register with branches etc (post stabilisation of the Autonomi API)
pub async fn handle_inspect_register(
    register_address: RegisterAddress,
    print_summary: bool,
    print_type: bool,
    print_size: bool,
    entries_range: Option<EntriesRange>,
    include_files: bool,
    files_args: FilesArgs,
) -> Result<bool> {
    let files_api = awe_client::connect_to_autonomi()
        .await
        .expect("Failed to connect to Autonomi Network");

    let result = files_api.client().get_register(register_address).await;
    let mut register = if result.is_ok() {
        result.unwrap()
    } else {
        return Err(eyre!("Error: register not found on network"));
    };

    register.sync(&mut files_api.wallet()?, true, None).await?;

    let entries_vec = helpers::node_entries_as_vec(&register);
    let size = entries_vec.len();
    if print_summary {
        do_print_summary(register.address(), &entries_vec, size)?;
    } else {
        if print_type {
            if size > 0 {
                do_print_type(Some(&entries_vec[0]))?;
            } else {
                do_print_type(None)?;
            }
        }

        if print_size {
            do_print_size(size)?;
        }
    }

    if let Some(entries_range) = entries_range {
        do_print_entries(
            &files_api,
            &entries_range,
            entries_vec,
            include_files,
            &files_args,
        )
        .await?;
    };

    Ok(true)
}

pub fn do_print_summary(
    reg_address: &RegisterAddress,
    entries_vec: &Vec<Entry>,
    size: usize,
) -> Result<bool> {
    println!("register: {}", reg_address.to_hex());
    if entries_vec.len() > 0 {
        do_print_type(Some(&entries_vec[0]))?;
    } else {
        do_print_type(None)?;
    }
    do_print_size(size)?;
    Ok(true)
}

pub fn do_print_type(reg_type: Option<&Entry>) -> Result<bool> {
    if reg_type.is_some() {
        let xor_name = helpers::xorname_from_entry(reg_type.unwrap());
        println!("type: {xor_name}");
    } else {
        println!("type: not set");
    }
    Ok(true)
}

pub fn do_print_size(size: usize) -> Result<bool> {
    println!("size: {size}");
    Ok(true)
}

pub async fn do_print_entries(
    files_api: &FilesApi,
    entries_range: &EntriesRange,
    entries_vec: Vec<Entry>,
    include_files: bool,
    files_args: &FilesArgs,
) -> Result<bool> {
    let size = entries_vec.len();
    if size == 0 { return Ok(true); }

    let first = if entries_range.start.is_some() {
        entries_range.start.unwrap()
    } else {
        0
    };

    let last = if entries_range.end.is_some() {
        entries_range.end.unwrap()
    } else {
        size - 1
    };

    if last > size - 1 {
        return Err(eyre!(
            "range exceeds maximum register entry which is {}",
            size - 1
        ));
    }

    // As entries_vec[] is in reverse order we adjust the start and end and count backwards
    println!("entries {first} to {last}:");
    for index in first..=last {
        let xor_name = helpers::xorname_from_entry(&entries_vec[index]);
        if include_files {
            println!("entry {index} - fetching metadata at {xor_name:64x}");
            match get_website_metadata_from_network(xor_name, &files_api).await {
                Ok(metadata) => {
                    let _ = do_print_files(&metadata, &files_args);
                }
                Err(e) => {
                    println!("Failed to get website metadata from network");
                    return Err(eyre!(e));
                }
            };
        } else {
            println!("{xor_name:64x}");
        }
    }

    Ok(true)
}

pub fn do_print_files(metadata: &WebsiteMetadata, files_args: &FilesArgs) -> Result<bool> {
    let metadata_stats = if files_args.print_metadata_summary || files_args.print_count_files {
        metadata_stats(metadata)?
    } else {
        (0 as usize, 0 as u64)
    };

    if files_args.print_metadata_summary {
        let _ = do_print_metadata_summary(metadata, metadata_stats);
    } else {
        if files_args.print_count_directories {
            let _ = do_print_count_directories(metadata);
        }

        if files_args.print_count_files {
            let _ = do_print_count_files(metadata_stats.0);
        }

        #[cfg(feature = "extra-file-metadata")]
        if files_args.print_total_bytes {
            let _ = do_print_total_bytes(metadata_stats.1);
        }
    }

    #[cfg(feature = "extra-file-metadata")]
    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in metadata.path_map.paths_to_files_map.iter() {
            for (file_name, chunk_address, modified, size) in path_map.iter() {
                if files_args.print_all_details {
                    let date_time = DateTime::<Utc>::from(*modified);
                    let modified_str = date_time.format("%Y-%m-%d %H:%M:%S").to_string();
                    println!(
                        "{:64x} {modified_str} \"{path_string}{file_name}\" {size} bytes",
                        chunk_address.xorname()
                    );
                } else {
                    println!(
                        "{:64x} \"{path_string}{file_name}\"",
                        chunk_address.xorname()
                    );
                }
            }
        }
    }

    #[cfg(not(feature = "extra-file-metadata"))]
    if files_args.print_paths || files_args.print_all_details {
        for (path_string, path_map) in metadata.path_map.paths_to_files_map.iter() {
            for (file_name, chunk_address) in path_map.iter() {
                if files_args.print_all_details {
                    // TODO add file metadata for 'details'
                    println!(
                        "{:64x} \"{path_string}{file_name}\"",
                        chunk_address.xorname()
                    );
                } else {
                    println!(
                        "{:64x} \"{path_string}{file_name}\"",
                        chunk_address.xorname()
                    );
                }
            }
        }
    }

    Ok(true)
}

#[cfg(feature = "extra-file-metadata")]
pub fn metadata_stats(metadata: &WebsiteMetadata) -> Result<(usize, u64)> {
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

#[cfg(not(feature = "extra-file-metadata"))]
pub fn metadata_stats(metadata: &WebsiteMetadata) -> Result<(usize, u64)> {
    let mut files_count: usize = 0;
    let total_bytes: u64 = 0;

    for (_, path_map) in metadata.path_map.paths_to_files_map.iter() {
        files_count = files_count + path_map.len();
    }

    Ok((files_count, total_bytes))
}

pub fn do_print_metadata_summary(
    metadata: &WebsiteMetadata,
    metadata_stats: (usize, u64),
) -> Result<bool> {
    println!("published  : {}", metadata.date_published);
    let _ = do_print_count_directories(metadata);
    let _ = do_print_count_files(metadata_stats.0);
    #[cfg(feature = "extra-file-metadata")]
    let _ = do_print_total_bytes(metadata_stats.1);
    Ok(true)
}

pub fn do_print_count_directories(metadata: &WebsiteMetadata) -> Result<bool> {
    println!(
        "directories: {}",
        metadata.path_map.paths_to_files_map.len()
    );
    Ok(true)
}

pub fn do_print_count_files(count_files: usize) -> Result<bool> {
    println!("files      : {count_files}");
    Ok(true)
}

#[cfg(feature = "extra-file-metadata")]
pub fn do_print_total_bytes(total_bytes: u64) -> Result<bool> {
    println!("total bytes: {total_bytes}");
    Ok(true)
}

/// Implement 'inspect-files' subcommand
///
/// Accepts a metadata address
///
/// TODO extend treatment to handle register with branches etc (post stabilisation of the Autonomi API)
pub async fn handle_inspect_files(
    metadata_address: XorName,
    files_args: FilesArgs,
) -> Result<bool> {
    let files_api = awe_client::connect_to_autonomi()
        .await
        .expect("Failed to connect to Autonomi Network");

    println!("fetching metadata at {metadata_address:64x}");
    match get_website_metadata_from_network(metadata_address, &files_api).await {
        Ok(metadata) => {
            let _ = do_print_files(&metadata, &files_args);
        }
        Err(e) => {
            println!("Failed to get website metadata from network");
            return Err(eyre!(e));
        }
    };
    Ok(true)
}
