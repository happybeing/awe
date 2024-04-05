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

use bytes::Bytes;
use color_eyre::Result;
use core::time::Duration;
use log::info;
use multiaddr::Multiaddr;
use sn_client::protocol::storage::ChunkAddress;
use sn_client::transfers::bls::SecretKey;
use sn_client::transfers::bls_secret_from_hex;
use sn_client::{Client, ClientEventsBroadcaster, FilesApi, FilesDownload};
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use xor_name::XorName;

const CLIENT_KEY: &str = "clientkey";

pub async fn connect_to_autonomi(
    peers: Vec<Multiaddr>,
    timeout: Option<Duration>,
) -> Result<Client> {
    println!("Autonomi client initialising...");
    let secret_key = get_client_secret_key(&get_client_data_dir_path()?)?;

    // let bootstrap_peers = get_peers_from_args(opt.peers).await?;

    println!("Connecting to the network using {} peers", peers.len(),);

    let bootstrap_peers = if peers.is_empty() {
        // empty vec is returned if `local-discovery` flag is provided
        None
    } else {
        Some(peers)
    };

    // get the broadcaster as we want to have our own progress bar.
    let broadcaster = ClientEventsBroadcaster::default();

    let result = Client::new(
        secret_key,
        bootstrap_peers,
        timeout,
        Some(broadcaster), // TODO try None
    )
    .await;

    Ok(result?)
}

// pub async fn old_connect_to_autonomi(peer: &Multiaddr) -> Result<Client> {
//     // note: this was pulled directly from sn_cli

//     println!("Initialising Autonomi client...");
//     let secret_key = get_client_secret_key(&get_client_data_dir_path()?)?;

//     let peer_args = PeersArgs {
//         first: false,
//         peers: vec![peer.clone()],
//     };
//     let bootstrap_peers = get_peers_from_args(peer_args).await?;

//     println!(
//         "Connecting to Autonomi with {} peer(s)",
//         bootstrap_peers.len(),
//     );

//     let bootstrap_peers = if bootstrap_peers.is_empty() {
//         // empty vec is returned if `local-discovery` flag is provided
//         None
//     } else {
//         Some(bootstrap_peers)
//     };

//     // get the broadcaster as we want to have our own progress bar.
//     let broadcaster = ClientEventsBroadcaster::default();

//     let result = Client::new(secret_key, bootstrap_peers, None, Some(broadcaster)).await?;
//     Ok(result)
// }

pub async fn autonomi_get_file(
    xor_name: XorName,
    files_api: &FilesApi,
) -> Result<Bytes, sn_client::Error> {
    let mut files_download = FilesDownload::new(files_api.clone());

    return match files_download
        .download_from(ChunkAddress::new(xor_name), 0, usize::MAX)
        .await
    {
        Ok(content) => Ok(content),
        Err(e) => Err(e),
    };
}

/// Get path to wallet_dir for this app, for use with sn_client::FilesApi
/// TODO post-demo, change to app specific wallet rather than sharing the Safe CLI wallet
pub fn get_client_data_dir_path() -> Result<PathBuf> {
    // note: this was pulled directly from sn_cli
    let mut home_dir = dirs_next::data_dir().expect("Data directory is obtainable");

    // TODO post-demo this will be the app name only and not include "client"
    home_dir.push("safe");
    home_dir.push("client");
    std::fs::create_dir_all(home_dir.as_path())?;
    info!("home_dir.as_path(): {}", home_dir.to_str().unwrap());
    Ok(home_dir)
}

pub fn str_to_xor_name(str: &String) -> Result<XorName> {
    let path = Path::new(str);
    let hex_xorname = path
        .file_name()
        .expect("Uploaded file to have name")
        .to_str()
        .expect("Failed to convert path to string");
    let bytes = hex::decode(hex_xorname)?;
    let xor_name_bytes: [u8; 32] = bytes
        .try_into()
        .expect("Failed to parse XorName from hex string");
    Ok(XorName(xor_name_bytes))
}

// Based on sn_cli
pub fn get_client_secret_key(root_dir: &PathBuf) -> Result<SecretKey> {
    // create the root directory if it doesn't exist
    std::fs::create_dir_all(root_dir)?;
    let key_path = root_dir.join(CLIENT_KEY);
    let secret_key = if key_path.is_file() {
        info!("Client key found. Loading from file...");
        let secret_hex_bytes = std::fs::read(key_path)?;
        bls_secret_from_hex(secret_hex_bytes)?
    } else {
        info!("No key found. Generating a new client key...");
        let secret_key = SecretKey::random();
        std::fs::write(key_path, hex::encode(secret_key.to_bytes()))?;
        secret_key
    };
    Ok(secret_key)
}
