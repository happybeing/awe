/*
*   Copyright (c) 2024 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU General Public License for more details.

*   You should have received a copy of the GNU General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod awe_client;
mod awe_protocols;
mod awe_subcommands;
mod awe_website_metadata;
mod awe_website_publisher;
mod awe_website_versions;
mod cli_options;

use color_eyre::Result;

// TODO fix messed up cursor keys. Only happens if I close window manually. Ctrl-C in terminal or CLI commands are fine
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let _ = awe_subcommands::cli_commands().await;
    Ok(())
}
