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
use color_eyre::eyre::{eyre, Result};
use std::path::PathBuf;
use xor_name::XorName;

use crate::dweb::helpers::autonomi::wallet::load_wallet;
use autonomi::client::archive::Archive;
use autonomi::client::Client;
use autonomi::Wallet;

use crate::dweb::data::awe_website_metadata::{osstr_to_string, JsonSettings, WebsiteMetadata};

/// Upload the website content and website metadata to Autonomi
/// TODO returns the xor address for the metadata used to access the website
pub async fn publish_website(
    website_root: &PathBuf,
    website_config_path: Option<PathBuf>,
    client: &Client,
    wallet: &Wallet,
) -> Result<XorName> {
    let website_config = if let Some(website_config_path) = website_config_path {
        match JsonSettings::load_json_file(&website_config_path) {
            Ok(config) => Some(config),
            Err(e) => {
                return Err(eyre!(
                    "Failed to load website config from {website_config_path:?}. {}",
                    e.root_cause()
                ));
            }
        }
    } else {
        None
    };

    match publish_website_content(client, website_root).await {
        Ok(site_upload_summary) => {
            match publish_website_metadata(
                client,
                website_root,
                &site_upload_summary,
                website_config,
                wallet,
            )
            .await
            {
                Ok(website_metadata) => Ok(website_metadata),
                Err(e) => Err(eyre!(
                    "Failed to store metadata for website: {}",
                    e.root_cause()
                )),
            }
        }
        Err(e) => Err(eyre!("Failed to store website content. {}", e.root_cause())),
    }
}

/// Uploads the tree of website content at website_root
/// Returns the autonomi::UploadSummary if all files are uploaded
pub async fn publish_website_content(client: &Client, website_root: &PathBuf) -> Result<Archive> {
    if !website_root.is_dir() {
        return Err(eyre!("Website path must be a directory: {website_root:?}"));
    }

    if !website_root.exists() {
        return Err(eyre!("Website path not found: {website_root:?}"));
    }

    if !website_root.read_dir().iter().len() == 0 {
        return Err(eyre!("Website path is empty: {website_root:?}"));
    }

    let wallet = load_wallet()?;
    println!("Uploading website from: {website_root:?}");
    let archive = match client.dir_upload(website_root.clone(), &wallet).await {
        Ok(archive) => archive,
        Err(e) => return Err(eyre!("Failed to upload directory tree: {e}")),
    };

    println!("web publish completed files: {:?}", archive.map().len());

    println!("WEBSITE CONTENT UPLOADED:");
    for (path, data_address, _metadata) in archive.iter() {
        println!("{data_address:64x} {path:?}");
    }

    Ok(archive)
}

// TODO review handling of errors that might occur here
// TODO consider extracting FilesApi::get_local_payment_and_upload_chunk() to StorageApi module
/// Creates metadata for a website using the upload_summary and website_settings
/// and stores it on Autonomi
/// Assumes paths are canonical
/// Returns the xor address of the stored metadata
pub async fn publish_website_metadata(
    client: &Client,
    website_root: &PathBuf,
    site_upload_archive: &Archive,
    website_config: Option<JsonSettings>,
    wallet: &Wallet,
) -> Result<XorName> {
    let mut metadata = WebsiteMetadata::new();
    if let Some(website_config) = website_config {
        metadata.website_config = website_config;
    };

    if let Some(website_root_string) = osstr_to_string(website_root.as_os_str()) {
        println!("DEBUG publish_website_metadata() website_root '{website_root_string}'");
        let resource_path_start = if website_root_string.ends_with(std::path::MAIN_SEPARATOR) {
            website_root_string.len() - 1
        } else {
            website_root_string.len()
        };

        for (full_path, datamap_address, _file_metadata) in site_upload_archive.iter() {
            if let Some(resource_full_path) = osstr_to_string(full_path.as_os_str()) {
                let resource_path = resource_full_path[resource_path_start..].to_string();
                println!("Adding '{resource_full_path}' as '{resource_path}'");
                match std::fs::metadata(resource_full_path) {
                    Ok(file_metadata) => metadata.add_resource_to_metadata(
                        &resource_path,
                        datamap_address.clone(),
                        Some(&file_metadata),
                    )?,
                    Err(e) => {
                        println!("Failed to obtain metadata for file due to: {e:}");
                        metadata.add_resource_to_metadata(
                            &resource_path,
                            datamap_address.clone(),
                            None,
                        )?
                    }
                };
            }
        }

        let xor_name = metadata
            .put_website_metadata_to_network(client.clone(), &wallet)
            .await?;
        println!("WEBSITE METADATA UPLOADED:\nawm://{xor_name:64x}");

        return Ok(xor_name);
    }

    return Err(eyre!("Invalid website root: '{website_root:?}'"));
}
