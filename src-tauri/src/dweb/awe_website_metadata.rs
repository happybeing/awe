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
use std::collections::HashMap;
use std::path::PathBuf;

use bytes::{BufMut, BytesMut};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use tauri::http::status::StatusCode;
use xor_name::XorName;

use autonomi::client::payment::PaymentOption;
use autonomi::client::Client;
use autonomi::Wallet;
use self_encryption::MAX_CHUNK_SIZE;

use crate::dweb::awe_client;

pub const PATH_SEPARATOR: char = '/';

pub async fn get_website_metadata_from_network(
    metadata_xor_name: XorName,
    client: &Client,
) -> Result<WebsiteMetadata> {
    println!("DEBUG get_website_metadata_from_network() at {metadata_xor_name:64x}");
    match awe_client::autonomi_get_file(metadata_xor_name, client).await {
        Ok(content) => {
            println!("Retrieved {} bytes", content.len());
            let metadata: WebsiteMetadata = rmp_serde::from_slice(&content)?;
            Ok(metadata)
        }

        Err(e) => {
            println!("FAILED: {e}");
            Err(e.into())
        }
    }
}

/// Manage settings as a JSON string in order to ensure serialisation and deserialisation
/// of WebsiteMetadata succeeds even as different settings are added or removed.
//
// This struct is used for two separate groups of settings. The first configure the
// website by defining redirects, overrides for default index files etc. The second is
// for awe and third-party application settings which are not needed in order to
// display the website itself, but may be used to change the behaviour of a client
// app when it accesses the website, or provide information about the client used
// to create or publish the site.
#[derive(Serialize, Deserialize)]
pub struct JsonSettings {
    json_string: String,
    // TODO implement non-serialised holder for JSON query object
}

impl JsonSettings {
    pub fn new() -> JsonSettings {
        JsonSettings {
            json_string: String::from(""),
        }
    }
    // TODO implement parsing to/from JSON query object
    // TODO implement setting/getting values using a hierarchy of keys

    /// Reads a JSON website configuration and returns a JSON query object
    /// TODO replace return type with a JSON query object holding settings
    pub fn load_json_file(_website_config: &PathBuf) -> Result<JsonSettings> {
        // TODO load_json_file()
        Ok(JsonSettings::new())
    }
}
#[derive(Serialize, Deserialize)]
pub struct WebsiteMetadata {
    /// System time of device publishing to Autonomi
    pub date_published: DateTime<Utc>,
    // TODO use website_config to implement web server like configuration such as redirects, short links and subdomains
    pub website_config: JsonSettings,
    // TODO document usage of third_party_settings JSON for metadata created by and accessible
    // TODO to unknown applications such as site builder, as well as awe. Mandate that:
    // TODO   Only a single application unique key per application be stored at the top level
    // TODO   and that applications only create values under their own key.
    // TODO   In the default provide "awe" as a top level key with no sub-values
    pub third_party_settings: JsonSettings,
    pub path_map: WebsitePathMap,

    index_filenames: Vec<String>, // Acceptable default index filenames (e.g. 'index.html')
}

impl WebsiteMetadata {
    pub fn new() -> WebsiteMetadata {
        WebsiteMetadata {
            date_published: Utc::now(),
            website_config: JsonSettings::new(),
            third_party_settings: JsonSettings::new(),
            path_map: WebsitePathMap::new(),
            index_filenames: Vec::from([String::from("index.html"), String::from("index.htm")]),
        }
    }

    /// Look up a canonicalised web resource path (which must begin with '/').
    /// If the path ends with '/' or no file matches a directory is assumed.
    /// For directories it will look for a default index file based on the site metadata settings.
    /// Returns a tuple with the resource's xor address if found, and a suitable HTTP response code.
    pub fn lookup_resource(&self, resource_path: &String) -> Result<XorName, StatusCode> {
        let last_separator_result = resource_path.rfind(PATH_SEPARATOR);
        if last_separator_result.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let original_resource_path = resource_path.clone();
        let mut resource_path = resource_path.clone();
        let last_separator = last_separator_result.unwrap();
        println!("Splitting path '{}'", resource_path);
        let second_part = resource_path.split_off(last_separator + 1);
        println!("...into '{}' and '{}'", resource_path, second_part);

        println!("Looking for resource at '{resource_path}'");
        if let Some(resources) = self.path_map.paths_to_files_map.get(&resource_path) {
            if second_part.len() > 0 {
                println!("DEBUG WebsiteMetadata looking up '{}'", second_part);
                match Self::lookup_name_in_vec(&second_part, &resources) {
                    Some(xorname) => return Ok(xorname),
                    None => {}
                }
            };
        };

        // Assume the second part is a directory name, so remake the path for that
        let new_resource_path = if original_resource_path.ends_with(PATH_SEPARATOR) {
            original_resource_path.clone()
        } else {
            original_resource_path.clone() + PATH_SEPARATOR.to_string().as_str()
        };

        println!("Retrying for index file in new_resource_path '{new_resource_path}'");
        if let Some(new_resources) = self.path_map.paths_to_files_map.get(&new_resource_path) {
            println!(
                "DEBUG looking for a default INDEX file, one of {:?}",
                self.index_filenames
            );
            // Look for a default index file
            for index_file in &self.index_filenames {
                // TODO might it be necessary to return the name of the resource?
                match Self::lookup_name_in_vec(&index_file, &new_resources) {
                    Some(xorname) => return Ok(xorname),
                    None => {}
                }
            }
        };

        println!("FAILED to find resource for path: '{original_resource_path}' in:");
        println!("{:?}", self.path_map.paths_to_files_map);

        Err(StatusCode::NOT_FOUND)
    }

    fn lookup_name_in_vec(
        name: &String,
        resources_vec: &Vec<(String, XorName, std::time::SystemTime, u64)>,
    ) -> Option<XorName> {
        for (resource_name, xor_name, _modified, _size) in resources_vec {
            if resource_name.eq(name) {
                return Some(xor_name.clone());
            }
        }
        None
    }

    /// Add a resource to the metadata map
    /// The resource_website_path MUST:
    /// - start with a forward slash denoting the website root
    /// - use forward slash to separate directories
    pub fn add_resource_to_metadata(
        &mut self,
        resource_website_path: &String,
        xor_name: XorName,
        file_metadata: Option<&std::fs::Metadata>,
    ) -> Result<()> {
        self.path_map
            .add_resource_to_metadata(resource_website_path, xor_name, file_metadata)
    }

    /// Upload website metadata and return the address (ie of the data map)
    pub async fn put_website_metadata_to_network(
        &self,
        client: Client,
        wallet: &Wallet,
    ) -> Result<XorName> {
        let serialised_metadata = rmp_serde::to_vec(self)?;
        if serialised_metadata.len() > MAX_CHUNK_SIZE {
            return Err(eyre!(
                "Failed to store website metadata - too large for one Chunk",
            ));
        }

        let mut bytes = BytesMut::with_capacity(MAX_CHUNK_SIZE);
        bytes.put(serialised_metadata.as_slice());

        match client
            .data_put(bytes.freeze(), PaymentOption::from(wallet))
            .await
        {
            Ok(data_map) => Ok(data_map),
            Err(e) => {
                let message = format!("Failed to upload website metadata - {e}");
                println!("{}", &message);
                return Err(eyre!(message.clone()));
            }
        }
    }
}

/// A map of paths to files used to access xor addresses of content
#[derive(Serialize, Deserialize)]
pub struct WebsitePathMap {
    // TODO add file metadata (size, date modified) using a struct: FileMeta {filename, size, date_modified, chunk_address}
    pub paths_to_files_map: HashMap<String, Vec<(String, XorName, std::time::SystemTime, u64)>>,
}

// TODO replace OS path separator with '/' when storing web paths
// TODO canonicalise path strings when adding them
impl WebsitePathMap {
    pub fn new() -> WebsitePathMap {
        WebsitePathMap {
            paths_to_files_map:
                HashMap::<String, Vec<(String, XorName, std::time::SystemTime, u64)>>::new(),
        }
    }

    /// Add a website resource to the metadata map
    /// resource_website_path MUST begin with a path separator denoting the website root
    /// This method handles translation of path separators
    pub fn add_resource_to_metadata(
        &mut self,
        resource_website_path: &String,
        xor_name: XorName,
        file_metadata: Option<&std::fs::Metadata>,
    ) -> Result<()> {
        // println!("DEBUG add_resource_to_metadata() path '{resource_website_path}'");
        let mut web_path = Self::webify_string(&resource_website_path);
        if let Some(last_separator_position) = web_path.rfind(PATH_SEPARATOR) {
            let resource_file_name = web_path.split_off(last_separator_position + 1);
            // println!(
            //     "DEBUG Splitting at {last_separator_position} into path: '{web_path}' file: '{resource_file_name}'"
            // );
            let metadata_tuple = if let Some(metadata) = file_metadata {
                (
                    resource_file_name.clone(),
                    xor_name,
                    metadata.modified().unwrap(),
                    metadata.len(),
                )
            } else {
                (
                    resource_file_name.clone(),
                    xor_name,
                    std::time::SystemTime::now(),
                    0,
                )
            };

            self.paths_to_files_map
                .entry(web_path)
                .and_modify(|vector| vector.push(metadata_tuple.clone()))
                .or_insert(vec![metadata_tuple]);
        } else {
            return Err(eyre!(
                "Path separator not found in resource website path: {resource_website_path}"
            ));
        }

        Ok(())
    }

    // Replace OS path separators with '/'
    // fn webify_path(path: &Path) -> String {
    //     match path.to_str() {
    //         Some(path_string) => {
    //             return Self::webify_string(&path_string.to_string());
    //         }
    //         None => {}
    //     }

    //     String::from("")
    // }

    // Replace OS path separators with '/'
    fn webify_string(path_string: &String) -> String {
        let path_string = path_string.clone();
        return path_string.replace(std::path::MAIN_SEPARATOR_STR, "/");
    }
}

pub fn osstr_to_string(file_name: &std::ffi::OsStr) -> Option<String> {
    if let Some(str) = file_name.to_str() {
        return Some(String::from(str));
    }
    None
}

// pub fn option_osstr_to_string(file_name: Option<&std::ffi::OsStr>) -> Option<String> {
//     if file_name.is_some() {
//         if let Some(str) = file_name.unwrap().to_str() {
//             return Some(String::from(str));
//         }
//     }
//     None
// }
