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

use autonomi::data::DataAddress;
use bytes::Bytes;
use color_eyre::eyre::{eyre, Result};
use xor_name::XorName;

use autonomi::PointerAddress as HistoryAddress;

use dweb::client::{ApiControl, DwebClient};
use dweb::helpers::convert::str_to_pointer_address;

use crate::awe_protocols::{AWE_PROTOCOL_DIRECTORY, AWE_PROTOCOL_FILE, AWE_PROTOCOL_HISTORY};
use crate::awe_subcommands::connect_and_announce;

/// Fallback for use by awe protocol handlers
pub async fn connect_to_autonomi() -> Result<DwebClient> {
    use crate::cli_options::Opt;
    use clap::Parser;
    let opt = Opt::parse();

    let api_control = ApiControl {
        tries: opt.retry_api,
        upload_file_by_file: opt.upload_file_by_file,
        ignore_pointers: opt.ignore_pointers,
        max_fee_per_gas: opt.max_fee_per_gas,
        ..Default::default()
    };

    let (client, _is_local_network) =
        connect_and_announce(opt.local, opt.alpha, api_control, true).await;
    Ok(client)
}

pub async fn is_local_network() -> bool {
    use crate::cli_options::Opt;
    use clap::Parser;
    let opt = Opt::parse();
    opt.local
}

pub async fn autonomi_get_file_public(
    client: &DwebClient,
    address: DataAddress,
) -> Result<Bytes, autonomi::client::GetError> {
    println!("DEBUG autonomi_get_file_public()");
    println!("DEBUG calling client.data_get_public()");
    match client.client.data_get_public(&address).await {
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
    let str = if str.starts_with(AWE_PROTOCOL_HISTORY) {
        &str[AWE_PROTOCOL_HISTORY.len()..]
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
    let str = if str.starts_with(AWE_PROTOCOL_HISTORY) {
        &str[AWE_PROTOCOL_HISTORY.len()..]
    } else {
        &str
    };

    match str_to_pointer_address(str) {
        Ok(pointer_address) => Ok(pointer_address),
        Err(e) => Err(eyre!("Invalid pointer address string '{str}':\n{e:?}")),
    }
}

pub fn awe_str_to_xor_name(str: &str) -> Result<XorName> {
    let str = if str.starts_with(AWE_PROTOCOL_DIRECTORY) {
        &str[AWE_PROTOCOL_DIRECTORY.len()..]
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
