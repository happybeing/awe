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

// TODO fix messed up cursor keys in terminal after running CLI command.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // color_eyre::install()?;
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    println!("DEBUG Hello from run()");

    // Windows doesn't attach a GUI application to the console so we
    // do it manually. This method doesn't cause the terminal input
    // to be blocked and so new commands can be entered while awe
    // sends output to the console. A blocking method is available,
    // but would require creation of a new terminal, which is not
    // suitable because awe is also a command line app.
    //
    // See: https://github.com/tauri-apps/tauri/issues/8305#issuecomment-1826871949
    //
    #[cfg(windows)]
    {
        use windows::Win32::System::Console::AllocConsole;
        let _ = unsafe { AllocConsole() };
    }

    let _ = awe_subcommands::cli_commands().await;
}
