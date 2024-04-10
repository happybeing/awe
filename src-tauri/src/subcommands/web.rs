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

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use clap::Parser;
use color_eyre::{eyre::bail, Result};
use walkdir::WalkDir;

use autonomi::{
    download_file, download_files, ChunkManager, Estimator, FilesUploadSummary, FilesUploader,
    UploadedFile, UPLOADED_FILES,
};

use sn_client::{Client, ClientEventsBroadcaster, FilesApi, UploadCfg, BATCH_SIZE};
use sn_protocol::storage::RetryStrategy;
use xor_name::XorName;

use crate::autonomi_websites::publish_website;

// Adapted from sn_cli::subcommands::files::files_cmds()

#[derive(Parser, Debug)]
pub enum WebCmds {
    /// Estimate the cost of uploading the website (excluding website metadata)
    Estimate {
        /// The root directory of website to upload.
        #[clap(value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        /// Should the website content be made accessible to all. (This is irreversible.)
        ///
        /// Note that without access to the website metadata even public data won't be
        /// discoverable. Access will only be possible with the address of the metadata
        /// or of the individual uploaded pages and resources contained in the metadata.
        #[clap(long, default_value = "true", short = 'p')]
        make_public: bool,
    },
    /// Publish a website.
    ///
    /// Uploads a tree of website files to the network along with metadata allowing
    /// the website to be accessed. (Pays using the default wallet).
    ///
    /// On successful upload prints the xor address of the website, accessible
    /// using Awe Browser using a URL like 'awex://<XOR-ADDRESS>'.
    Publish {
        /// The root directory of the website to be published.
        #[clap(value_name = "WEBSITE-ROOT")]
        website_root: PathBuf,
        /// Optional website configuration such as default index file, redirects etc.
        #[clap(long = "website_config", short = 'c', value_name = "JSON-FILE")]
        website_config: Option<PathBuf>,
        /// The batch_size to split chunks into parallel handling batches
        /// during payment and upload processing.
        #[clap(long, default_value_t = sn_client::BATCH_SIZE, short='b')]
        batch_size: usize,
        /// Should the website content be made accessible to all. (This is irreversible.)
        ///
        /// Note that without access to the website metadata even public data won't be
        /// discoverable. Access will only be possible with the address of the metadata
        /// or of the individual uploaded pages and resources contained in the metadata.
        #[clap(long, name = "make_public", default_value = "true", short = 'p')]
        make_public: bool,
        /// Set the strategy to use on chunk upload failure. Does not modify the spend failure retry attempts yet.
        ///
        /// Choose a retry strategy based on effort level, from 'quick' (least effort), through 'balanced',
        /// to 'persistent' (most effort).
        #[clap(long, default_value_t = RetryStrategy::Balanced, short = 'r', help = "Sets the retry strategy on upload failure. Options: 'quick' for minimal effort, 'balanced' for moderate effort, or 'persistent' for maximum effort.")]
        retry_strategy: RetryStrategy,
    },
    // Download {
    //     /// The name to apply to the downloaded file.
    //     ///
    //     /// If the name argument is used, the address argument must also be supplied.
    //     ///
    //     /// If neither are, all the files uploaded by the current user will be downloaded again.
    //     #[clap(name = "name")]
    //     file_name: Option<OsString>,
    //     /// The hex address of a file.
    //     ///
    //     /// If the address argument is used, the name argument must also be supplied.
    //     ///
    //     /// If neither are, all the files uploaded by the current user will be downloaded again.
    //     #[clap(name = "address")]
    //     file_addr: Option<String>,
    //     /// Flagging whether to show the holders of the uploaded chunks.
    //     /// Default to be not showing.
    //     #[clap(long, name = "show_holders", default_value = "false")]
    //     show_holders: bool,
    //     /// The batch_size for parallel downloading
    //     #[clap(long, default_value_t = BATCH_SIZE , short='b')]
    //     batch_size: usize,
    //     /// Set the strategy to use on downloads failure.
    //     ///
    //     /// Choose a retry strategy based on effort level, from 'quick' (least effort), through 'balanced',
    //     /// to 'persistent' (most effort).
    //     #[clap(long, default_value_t = RetryStrategy::Quick, short = 'r', help = "Sets the retry strategy on download failure. Options: 'quick' for minimal effort, 'balanced' for moderate effort, or 'persistent' for maximum effort.")]
    //     retry_strategy: RetryStrategy,
    // },
}

pub(crate) async fn web_cmds(
    cmds: WebCmds,
    client: &Client,
    root_dir: &Path,
    verify_store: bool,
) -> Result<()> {
    match cmds {
        WebCmds::Estimate {
            website_root,
            make_public,
        } => {
            let files_api = FilesApi::build(client.clone(), root_dir.to_path_buf())?;
            let chunk_manager = ChunkManager::new(root_dir);
            Estimator::new(chunk_manager, files_api)
                .estimate_cost(website_root, make_public, root_dir)
                .await?
        }
        WebCmds::Publish {
            website_root,
            website_config,
            make_public,
            batch_size,
            retry_strategy,
        } => {
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

            let upload_config = UploadCfg {
                batch_size,
                verify_store,
                retry_strategy,
                ..Default::default()
            };

            publish_website(
                &website_root,
                website_config,
                make_public,
                client,
                root_dir,
                &upload_config,
            )
            .await;
        } // WebCmds::Download {
          //     file_name,
          //     file_addr,
          //     show_holders,
          //     batch_size,
          //     retry_strategy,
          // } => {
          //     if (file_name.is_some() && file_addr.is_none())
          //         || (file_addr.is_some() && file_name.is_none())
          //     {
          //         return Err(
          //             eyre!("Both the name and address must be supplied if either are used")
          //                 .suggestion(
          //                 "Please run the command again in the form 'files upload <name> <address>'",
          //             ),
          //         );
          //     }

          //     let mut download_dir = root_dir.to_path_buf();
          //     let mut download_file_name = file_name.clone();
          //     if let Some(file_name) = file_name {
          //         // file_name may direct the downloaded data to:
          //         //
          //         // the current directory (just a filename)
          //         // eg safe files download myfile.txt ADDRESS
          //         //
          //         // a directory relative to the current directory (relative filename)
          //         // eg safe files download my/relative/path/myfile.txt ADDRESS
          //         //
          //         // a directory relative to root of the filesystem (absolute filename)
          //         // eg safe files download /home/me/mydir/myfile.txt ADDRESS
          //         let file_name_path = Path::new(&file_name);
          //         if file_name_path.is_dir() {
          //             return Err(eyre!("Cannot download file to path: {:?}", file_name));
          //         }
          //         let file_name_dir = file_name_path.parent();
          //         if file_name_dir.is_none() {
          //             // just a filename, use the current_dir
          //             download_dir = std::env::current_dir().unwrap_or(root_dir.to_path_buf());
          //         } else if file_name_path.is_relative() {
          //             // relative to the current directory. Make the relative path
          //             // into an absolute path by joining it to current_dir
          //             if let Some(relative_dir) = file_name_dir {
          //                 let current_dir = std::env::current_dir().unwrap_or(root_dir.to_path_buf());
          //                 download_dir = current_dir.join(relative_dir);
          //                 if !download_dir.exists() {
          //                     return Err(eyre!("Directory does not exist: {:?}", download_dir));
          //                 }
          //                 if let Some(path_file_name) = file_name_path.file_name() {
          //                     download_file_name = Some(OsString::from(path_file_name));
          //                 }
          //             }
          //         } else {
          //             // absolute dir
          //             download_dir = file_name_dir.unwrap_or(root_dir).to_path_buf();
          //         }
          //     }
          //     let files_api: FilesApi = FilesApi::new(client.clone(), download_dir.clone());

          //     match (download_file_name, file_addr) {
          //         (Some(download_file_name), Some(address_provided)) => {
          //             let bytes =
          //                 hex::decode(&address_provided).expect("Input address is not a hex string");
          //             let xor_name_provided = XorName(
          //                 bytes
          //                     .try_into()
          //                     .expect("Failed to parse XorName from hex string"),
          //             );
          //             // try to read the data_map if it exists locally.
          //             let uploaded_files_path = root_dir.join(UPLOADED_FILES);
          //             let expected_data_map_location = uploaded_files_path.join(address_provided);
          //             let local_data_map = {
          //                 if expected_data_map_location.exists() {
          //                     let uploaded_file_metadata =
          //                         UploadedFile::read(&expected_data_map_location)?;

          //                     uploaded_file_metadata.data_map.map(|bytes| Chunk {
          //                         address: ChunkAddress::new(xor_name_provided),
          //                         value: bytes,
          //                     })
          //                 } else {
          //                     None
          //                 }
          //             };

          //             download_file(
          //                 files_api,
          //                 xor_name_provided,
          //                 (download_file_name, local_data_map),
          //                 &download_dir,
          //                 show_holders,
          //                 batch_size,
          //                 retry_strategy,
          //             )
          //             .await
          //         }
          //         _ => {
          //             println!("Attempting to download all files uploaded by the current user...");
          //             download_files(
          //                 &files_api,
          //                 root_dir,
          //                 show_holders,
          //                 batch_size,
          //                 retry_strategy,
          //             )
          //             .await?
          //         }
          //     }
          // }
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
