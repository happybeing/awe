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
use autonomi::{
    download_file, download_files, ChunkManager, Estimator, FilesUploadSummary, FilesUploader,
    UploadedFile, UPLOADED_FILES,
};

use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use sn_client::{Client, ClientEventsBroadcaster, FilesApi, UploadCfg, BATCH_SIZE};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xor_name::XorName;

#[derive(Serialize, Deserialize)]
pub struct WebsiteMetadata {
    // TODO implement web server like configuration such as redirects
    // TODO provide a method for versioning of this structure which allows older versions to be parsed
    // TODO provide for optional metadata (possibly encrypted), which is ignored by this module.
    // TODO  Such as metadata created by and accessible to a site builder.
    path_map: HashMap<PathBuf, XorName>,
}

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
    // Err(eyre!("NOTING"))//Ok(files_upload_summary)
}

/// Creates metadata for a website using the upload_summary and website_settings
/// and stores it on Autonomi
/// Returns the xor address of the stored summary
/// TODO everything!
pub async fn publish_website_metadata(
    client: &Client,
    root_dir: &Path,
    site_upload_summary: &FilesUploadSummary,
    make_public: bool,
    website_settings: Option<&WebsiteMetadata>, // TODO change to the JSON query object when implemented
    upload_cfg: &UploadCfg,
) -> Result<()> {
    Ok(())
}
