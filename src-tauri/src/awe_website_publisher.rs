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
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xor_name::XorName;

use autonomi::{FilesUploadSummary, FilesUploader};
use self_encryption::MAX_CHUNK_SIZE;
use sn_client::{Client, ClientEventsBroadcaster, FilesApi, UploadCfg, BATCH_SIZE};
use sn_protocol::storage::Chunk;

use crate::awe_website_metadata::osstr_to_string;

use super::awe_website_metadata::WebsiteMetadata;

/// Upload the website content and website metadata to Autonomi
/// TODO returns the xor address for the metadata used to access the website
pub async fn publish_website(
    website_root: &PathBuf,
    website_config: Option<PathBuf>,
    make_public: bool,
    client: &Client,
    root_dir: &Path,
    upload_config: &UploadCfg,
) {
    let website_settings = None;
    if let Some(website_config) = website_config {
        match load_website_config(&website_config) {
            Ok(result) => {
                let mut website_settings = None;
                let ws;
                if result.is_some() {
                    ws = result.unwrap();
                    website_settings = Some(&ws)
                }
            }
            Err(e) => {
                println!("Failed to load website config: {e:?}");
                return;
            }
        }
    }

    match publish_website_content(
        client,
        root_dir,
        website_root,
        make_public,
        website_settings,
        &upload_config,
    )
    .await
    {
        Ok(site_upload_summary) => {
            match publish_website_metadata(
                client,
                root_dir,
                website_root,
                &site_upload_summary,
                make_public,
                website_settings,
                &upload_config,
            )
            .await
            {
                Ok(metadata_summary) => {}
                Err(e) => {
                    println!("Failed to store metadata for website: {e:?}")
                }
            }
        }
        Err(e) => {
            println!("Failed to store website content: {e:?}")
        }
    };
}

/// Reads a JSON website configuration and returns a JSON query object
/// TODO replace return type with a JSON query object holding settings
pub fn load_website_config(website_config: &PathBuf) -> Result<Option<WebsiteMetadata>> {
    // TODO load_website_config()
    Ok(None)
}

/// Uploads the tree of website content at website_root
/// Returns the autonomi::FilesUploadSummary if all files are uploaded
pub async fn publish_website_content(
    client: &Client,
    root_dir: &Path,
    website_root: &PathBuf,
    make_public: bool,
    website_settings: Option<&WebsiteMetadata>,
    upload_cfg: &UploadCfg,
) -> Result<FilesUploadSummary> {
    if !website_root.is_dir() {
        return Err(eyre!("Website path must be a directory"));
    }

    if !website_root.exists() {
        return Err(eyre!("Website path not found"));
    }

    if !website_root.read_dir().iter().len() == 0 {
        return Err(eyre!("Website path is empty"));
    }

    // TODO load website_config and use to:
    // TODO   override defaults (such as make_public)
    // TODO   provide settings for website
    let files_uploader = FilesUploader::new(client.clone(), root_dir.to_path_buf())
        .set_make_data_public(make_public)
        .set_upload_cfg(*upload_cfg)
        .insert_path(&website_root);

    let files_upload_summary = files_uploader.start_upload().await?;

    // TODO replace this with code to store website metadata and print its XorAddress
    println!(
        "web publish completed files: {:?}",
        files_upload_summary.completed_files
    );

    let completed_files = files_upload_summary.completed_files.clone();
    println!("WEBSITE CONTENT UPLOADED:");
    for (path, _, chunk_address) in completed_files {
        println!("{} {path:?}", chunk_address.to_hex());
    }

    Ok(files_upload_summary)
}

// TODO review handling of errors that might occur here
// TODO consider extracting FilesApi::get_local_payment_and_upload_chunk() to StorageApi module
/// Creates metadata for a website using the upload_summary and website_settings
/// and stores it on Autonomi
/// Assumes paths are canonical
/// Returns the xor address of the stored metadata
pub async fn publish_website_metadata(
    client: &Client,
    root_dir: &Path,
    website_root: &PathBuf,
    site_upload_summary: &FilesUploadSummary,
    make_public: bool,
    website_settings: Option<&WebsiteMetadata>, // TODO change to the JSON query object when implemented
    upload_cfg: &UploadCfg,
) -> Result<XorName> {
    let mut metadata = WebsiteMetadata::new();

    if let Some(website_root_string) = osstr_to_string(website_root.as_os_str()) {
        println!("DEBUG publish_website_metadata() website_root '{website_root_string}'");
        let resource_path_start = if website_root_string.ends_with(std::path::MAIN_SEPARATOR) {
            website_root_string.len() - 1
        } else {
            website_root_string.len()
        };

        for (full_path, _file_name, chunk_address) in
            site_upload_summary.completed_files.clone().into_iter()
        {
            if let Some(resource_full_path) = osstr_to_string(full_path.as_os_str()) {
                let resource_path = resource_full_path[resource_path_start..].to_string();
                println!("Adding '{resource_full_path}' as '{resource_path}'");
                metadata.add_resource_to_metadata(&resource_path, chunk_address.clone())?
            }
        }

        let xor_name = metadata
            .put_website_metadata_to_network(client.clone(), root_dir, upload_cfg)
            .await?;
        println!("WEBSITE METADATA UPLOADED:\n{xor_name:64x}");

        return Ok(xor_name);
    }

    return Err(eyre!("Invalid website root: '{website_root:?}'"));
}
