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

use std::convert::TryInto;

use bytes::Bytes;
use color_eyre::{eyre::eyre, Result};
use xor_name::XorName;

use autonomi::client::data::GetError;
use autonomi::client::Client;
use sn_registers::RegisterAddress;
// TODO remove?
// use autonomi::client::data::
// use sn_client::transfers::bls::SecretKey;
// use sn_client::transfers::bls_secret_from_hex;
// use sn_client::{Client, ClientEventsBroadcaster, FilesApi, FilesDownload};

use crate::awe_protocols::{AWE_PROTOCOL_FILE, AWE_PROTOCOL_METADATA, AWE_PROTOCOL_REGISTER};

pub async fn connect_to_autonomi() -> Result<Client> {
    println!("Autonomi client initialising...");
    crate::dweb::helpers::autonomi::connect::connect_to_network().await
}

pub async fn autonomi_get_file(xor_name: XorName, client: &Client) -> Result<Bytes, GetError> {
    println!("DEBUG autonomi_get_file()");
    println!("DEBUG calling client.data_get()");
    match client.data_get(xor_name).await {
        Ok(content) => {
            println!("DEBUG Ok() return");
            Ok(content)
        }
        Err(e) => {
            println!("DEBUG Err() return");
            Err(e)
        }
    }
}

// The following functions copied from sn_cli with minor changes (eg to message text)

/// Get path to wallet_dir for this app, for use with sn_client::FilesApi
/// TODO post-demo, change to app specific wallet rather than sharing the Safe CLI wallet
// TODO now obtained from helpers::autonomi::access::user_data.rs
// pub fn get_client_data_dir_path() -> Result<PathBuf> {
//     // note: this was pulled directly from sn_cli
//     let mut home_dir = dirs_next::data_dir().expect("Data directory is obtainable");

//     // TODO post-demo this will be the app name only and not include "client"
//     home_dir.push("safe");
//     home_dir.push("client");
//     std::fs::create_dir_all(home_dir.as_path())?;
//     info!("home_dir.as_path(): {}", home_dir.to_str().unwrap());
//     Ok(home_dir)
// }

/// Parse a hex register address with optional URL scheme
pub fn str_to_register_address(str: &str) -> Result<RegisterAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_REGISTER) {
        &str[AWE_PROTOCOL_REGISTER.len()..]
    } else {
        &str
    };

    match RegisterAddress::from_hex(str) {
        Ok(register_address) => Ok(register_address),
        Err(e) => Err(eyre!("Invalid register address string '{str}':\n{e:?}")),
    }
}

/// Parse a hex xor address with optional URL scheme
pub fn str_to_xor_name(str: &str) -> Result<XorName> {
    let mut str = if str.starts_with(AWE_PROTOCOL_METADATA) {
        &str[AWE_PROTOCOL_METADATA.len()..]
    } else if str.starts_with(AWE_PROTOCOL_FILE) {
        &str[AWE_PROTOCOL_FILE.len()..]
    } else {
        &str
    };
    str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}

// Based on sn_cli
// TODO remove?
// pub fn get_client_secret_key(root_dir: &PathBuf) -> Result<SecretKey> {
//     // create the root directory if it doesn't exist
//     std::fs::create_dir_all(root_dir)?;
//     let key_path = root_dir.join(CLIENT_KEY);
//     let secret_key = if key_path.is_file() {
//         info!("Client key found. Loading from file...");
//         let secret_hex_bytes = std::fs::read(key_path)?;
//         bls_secret_from_hex(secret_hex_bytes)?
//     } else {
//         info!("No key found. Generating a new client key...");
//         let secret_key = SecretKey::random();
//         std::fs::write(key_path, hex::encode(secret_key.to_bytes()))?;
//         secret_key
//     };
//     Ok(secret_key)
// }
