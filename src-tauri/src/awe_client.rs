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

use bytes::Bytes;
use color_eyre::eyre::{eyre, Result};
use xor_name::XorName;

use ant_bootstrap::PeersArgs;
use ant_protocol::storage::PointerAddress as HistoryAddress;

use dweb::client::AutonomiClient;
use dweb::helpers::convert::str_to_pointer_address;

use crate::awe_protocols::{AWE_PROTOCOL_FILE, AWE_PROTOCOL_METADATA, AWE_PROTOCOL_REGISTER};

pub async fn connect_to_autonomi() -> Result<AutonomiClient> {
    println!("Autonomi client initialising...");
    dweb::client::AutonomiClient::initialise_and_connect(Some(PeersArgs::default())).await
}

pub async fn autonomi_get_file_public(
    client: &AutonomiClient,
    xor_name: XorName,
) -> Result<Bytes, autonomi::client::GetError> {
    println!("DEBUG autonomi_get_file_public()");
    println!("DEBUG calling client.data_get_public()");
    match client.data_get_public(xor_name).await {
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

/// Parse a hex HistoryAddress with optional URL scheme
pub fn awe_str_to_history_address(str: &str) -> Result<HistoryAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_REGISTER) {
        &str[AWE_PROTOCOL_REGISTER.len()..]
    } else {
        &str
    };

    match str_to_pointer_address(str) {
        Ok(history_address) => Ok(history_address),
        Err(e) => Err(eyre!(
            "Invalid History (pointer) address string '{str}':\n{e:?}"
        )),
    }
}

/// Parse a hex PointerAddress with optional URL scheme
pub fn awe_str_to_pointer_address(str: &str) -> Result<HistoryAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_REGISTER) {
        &str[AWE_PROTOCOL_REGISTER.len()..]
    } else {
        &str
    };

    match str_to_pointer_address(str) {
        Ok(pointer_address) => Ok(pointer_address),
        Err(e) => Err(eyre!("Invalid pointer address string '{str}':\n{e:?}")),
    }
}

pub fn awe_str_to_xor_name(str: &str) -> Result<XorName> {
    let str = if str.starts_with(AWE_PROTOCOL_METADATA) {
        &str[AWE_PROTOCOL_METADATA.len()..]
    } else if str.starts_with(AWE_PROTOCOL_FILE) {
        &str[AWE_PROTOCOL_FILE.len()..]
    } else {
        &str
    };
    let str = if str.ends_with('/') {
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
