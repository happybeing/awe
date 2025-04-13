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
use blsttc::PublicKey;
use chrono::offset::Utc;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Result};
use std::time::{Duration, UNIX_EPOCH};

use autonomi::client::key_derivation::{DerivationIndex, MainPubkey};
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::{GraphEntry, GraphEntryAddress, Pointer, PointerAddress};

use dweb::client::DwebClient;
use dweb::files::directory::Tree;
use dweb::helpers::graph_entry::graph_entry_get;
use dweb::trove::History;
use dweb::trove::HistoryAddress;

use crate::cli_options::{EntriesRange, FilesArgs};

/// Implement 'inspect-history' subcommand
pub async fn handle_inspect_history(
    _client: DwebClient,
    _history_address: HistoryAddress,
    _print_history_full: bool,
    _entries_range: Option<EntriesRange>,
    _include_files: bool,
    _graph_keys: bool,
    _shorten_hex_strings: bool,
    _files_args: FilesArgs,
) -> Result<()> {
    println!("This awe subcommand is deprecated but you can 'cargo install dweb' and use dweb's subcommands instead");
    Ok(())
}

/// Implement 'inspect-pointer' subcommand
pub async fn handle_inspect_pointer(
    _client: DwebClient,
    _pointer_address: PointerAddress,
) -> Result<()> {
    println!("This awe subcommand is deprecated but you can 'cargo install dweb' and use dweb's subcommands instead");
    Ok(())
}

fn print_pointer(pointer: &Pointer, pointer_address: &PointerAddress) {
    println!("pointer     : {}", pointer_address.to_hex());
    println!("  target    : {:x}", pointer.target().xorname());
    println!("  counter   : {}", pointer.counter());
}

/// Implement 'inspect-graphentry' subcommand
pub async fn handle_inspect_graphentry(
    _client: DwebClient,
    _graph_entry_address: GraphEntryAddress,
    _full: bool,
    _shorten_hex_strings: bool,
) -> Result<()> {
    println!("This awe subcommand is deprecated but you can 'cargo install dweb' and use dweb's subcommands instead");
    Ok(())
}

/// Implement 'inspect-files' subcommand
pub async fn handle_inspect_files(
    _client: DwebClient,
    _archive_address: ArchiveAddress,
    _files_args: FilesArgs,
) -> Result<()> {
    println!("This awe subcommand is deprecated but you can 'cargo install dweb' and use dweb's subcommands instead");
    Ok(())
}
