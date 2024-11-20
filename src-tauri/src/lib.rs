/*
*   Copyright (c) 2024-2025 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU Affero General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU Affero General Public License for more details.

*   You should have received a copy of the GNU Affero General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dweb;

mod awe_client;
mod awe_const;
mod awe_protocols;
mod cli_options;
mod commands;
mod generated_rs;

use sn_logging::{Level, LogBuilder};

use crate::commands::awe_subcommands;

// TODO fix messed up cursor keys in terminal after running CLI command.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // color_eyre::install()?;
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    use crate::cli_options::Opt;
    use clap::Parser;
    let opt = Opt::parse();

    if opt.client_logs {
        let logging_targets = vec![
            // TODO: Reset to nice and clean defaults once we have a better idea of what we want
            ("sn_networking".to_string(), Level::DEBUG),
            ("safe".to_string(), Level::TRACE),
            ("sn_build_info".to_string(), Level::TRACE),
            ("autonomi".to_string(), Level::TRACE),
            ("sn_client".to_string(), Level::TRACE),
            ("sn_logging".to_string(), Level::TRACE),
            ("sn_peers_acquisition".to_string(), Level::TRACE),
            ("sn_protocol".to_string(), Level::TRACE),
            ("sn_registers".to_string(), Level::TRACE),
            ("sn_transfers".to_string(), Level::TRACE),
        ];

        let log_builder = LogBuilder::new(logging_targets);
        // log_builder.output_dest(opt.log_output_dest);
        // log_builder.format(opt.log_format.unwrap_or(LogFormat::Default));
        let _log_handles = log_builder.initialize().unwrap();
    };

    // Windows doesn't attach a GUI application to the console so we
    // do it manually - but only when the GUI is to be activated.
    //
    // This method doesn't cause the terminal input to be blocked
    // and so new commands can be entered while the awe GUI sends
    // output to the console. A blocking method is available, but
    // would require creation of a new terminal, which is not
    // suitable because awe is also a command line app.
    //
    // See: https://github.com/tauri-apps/tauri/issues/8305#issuecomment-1826871949
    //
    #[cfg(windows)]
    {
        if match opt.cmd {
            Some(cli_options::Subcommands::Browse {
                url: _,
                website_version: _,
            }) => true,
            Some(_) => false,
            None => true,
        } {
            let _ = unsafe { windows::Win32::System::Console::AllocConsole() };
        }
    };

    let url = opt.url.clone();
    let website_version = opt.website_version.clone();

    if tauri::async_runtime::block_on(async move {
        awe_subcommands::cli_commands(opt)
            .await
            .is_ok_and(|complete| !complete)
    }) {
        // No command complete, so register protocols and open the browser
        crate::awe_protocols::register_protocols(url, website_version);
    };
}
