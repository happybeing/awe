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
use ant_registers::RegisterAddress;
use autonomi::client::data::GetError;
use autonomi::client::Client;

use dweb::autonomi::access::network::get_peers;

use crate::awe_protocols::{AWE_PROTOCOL_FILE, AWE_PROTOCOL_METADATA, AWE_PROTOCOL_REGISTER};

pub async fn connect_to_autonomi() -> Result<Client> {
    println!("Autonomi client initialising...");
    crate::connect::connect_to_network(get_peers(PeersArgs::default()).await?).await
}

pub async fn autonomi_get_file_public(
    xor_name: XorName,
    client: &Client,
) -> Result<Bytes, GetError> {
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

/// Parse a hex register address with optional URL scheme
pub fn awe_str_to_register_address(str: &str) -> Result<RegisterAddress> {
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
pub fn awe_str_to_xor_name(str: &str) -> Result<XorName> {
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

    println!("DEBUG hex::decode({str})");
    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}
