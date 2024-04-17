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

use clap::Parser;
use tauri::http::{status::StatusCode, Request, Response};

use sn_client::{
    networking::{GetRecordError, NetworkError},
    Client, FilesApi,
};
use sn_peers_acquisition::get_peers_from_args;

use crate::awe_client;
use crate::cli_options::Opt;

// TODO See if I can instantiate a client and other startup things as static and share
// TODO across handlers and requests. For now I make new client etc. for every request.
// TODO So maybe make some objects static? (app_config / client / files_api)
//
// TODO review technique used to call async within protocol handlers (discouraged here: https://stackoverflow.com/questions/66035290/how-do-i-await-a-future-inside-a-non-async-method-which-was-called-from-an-async)
// TODO Tauri is supposed to support calling async inside protocol handler, but I didn't find a recommended method
// TODO ASKED Tauri Discord: https://discord.com/channels/616186924390023171/1224053280565497896
// TODO ASKED Stack Overflow:https://stackoverflow.com/questions/78255320/what-is-the-recommended-technique-for-calling-an-async-function-within-a-tauri-p
// TODO Notes: Discord solution re Tauri 2.0 - thought I was using v2.0?
// TODO Notes: Discord solution suggests I can use spawn after all, worth trying as this blocks for ages
pub async fn register_protocols() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("xor", move |_app, req| {
            let handle = tokio::runtime::Handle::current();
            let _guard = handle.enter();
            // initialise safe network connection and files api
            let client = futures::executor::block_on(async move {
                let opt = Opt::parse();
                let peers = get_peers_from_args(opt.peers).await?;
                let timeout = opt.connection_timeout;
                awe_client::connect_to_autonomi(peers, timeout).await
            })
            .expect("Failed to connect to Autonomi Network");
            let wallet_dir =
                awe_client::get_client_data_dir_path().expect("Failed to get client data dir path");
            let files_api = FilesApi::build(client.clone(), wallet_dir)
                .expect("Failed to instantiate FilesApi");
            futures::executor::block_on(async move {
                crate::awe_protocols::handle_protocol_xor(&req, &files_api.clone()).await
            })
        })
        .register_uri_scheme_protocol("awex", move |_app, req| {
            let handle = tokio::runtime::Handle::current();
            let _guard = handle.enter();
            let client = futures::executor::block_on(async move {
                let opt = Opt::parse();
                let peers = get_peers_from_args(opt.peers).await?;
                let timeout = opt.connection_timeout;
                awe_client::connect_to_autonomi(peers, timeout).await
            })
            .expect("Failed to connect to Autonomi Network");
            let wallet_dir =
                awe_client::get_client_data_dir_path().expect("Failed to get client data dir path");
            let files_api = FilesApi::build(client.clone(), wallet_dir)
                .expect("Failed to instantiate FilesApi");
            futures::executor::block_on(async move {
                crate::awe_protocols::handle_protocol_awex(&req, &files_api.clone()).await
            })
        })
        .register_uri_scheme_protocol("awe", move |_app, req| {
            let handle = tokio::runtime::Handle::current();
            let _guard = handle.enter();
            let client = futures::executor::block_on(async move {
                let opt = Opt::parse();
                let peers = get_peers_from_args(opt.peers).await?;
                let timeout = opt.connection_timeout;
                awe_client::connect_to_autonomi(peers, timeout).await
            })
            .expect("Failed to connect to Autonomi Network");
            let wallet_dir =
                awe_client::get_client_data_dir_path().expect("Failed to get client data dir path");
            let files_api = FilesApi::build(client.clone(), wallet_dir)
                .expect("Failed to instantiate FilesApi");
            futures::executor::block_on(async move {
                crate::awe_protocols::handle_protocol_awe(&req, &files_api.clone()).await
            })
        })
        // This does nothing:
        // TODO try using CSP to block other protocols. Review this: https://stackoverflow.com/questions/77806138/what-is-the-correct-way-to-configure-csp-in-tauri-when-using-css-in-js-libraries
        .register_uri_scheme_protocol("http", |_app, req| {
            println!("http-scheme: {req:?}");
            tauri::http::ResponseBuilder::new().body(Vec::new())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use crate::awe_website_metadata::{get_website_metadata_from_network, PATH_SEPARATOR};
use xor_name::XorName;

// TODO implement publishing via version register (based on webname)
// TODO Placeholder for awe:// webname protocol
/// Fetch data using a webname URL
/// Returns content as an http Response
async fn handle_protocol_awe(
    req: &Request,
    _files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Hello from handle_protocol_awe()");
    let url = req.uri();
    let content =
        format!("<HTML><HEAD></HEAD><BODY><h1>Handling Autonomi Request</h1>{url:?}</BODY></HTML>");
    tauri::http::ResponseBuilder::new().body(content.into_bytes())
}

const PROTOCOL_AWEX: &str = "awex://";

/// Fetch data using an xor URL (awex:// protocol address and optional path)
/// Returns content as an http Response
async fn handle_protocol_awex(
    req: &Request,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Hello from handle_protocol_awex()");

    let url = req.uri();
    println!("DEBUG url '{url}'");
    let (_, remainder) = if url.starts_with(PROTOCOL_AWEX) {
        url.split_at(PROTOCOL_AWEX.len())
    } else {
        ("", url)
    };

    let mut remainder = remainder.to_string();
    let (xor_string, resource_path) = match remainder.find(PATH_SEPARATOR) {
        Some(separator_position) => {
            let path_part = remainder.split_off(separator_position);
            (remainder, path_part)
        }
        None => (remainder, String::from(PATH_SEPARATOR)),
    };

    println!("DEBUG (xor_string, resource_path): ({xor_string}, {resource_path})'");
    let xor_name = match awe_client::str_to_xor_name(&xor_string) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address [{:?}]", err);
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes());
        }
    };

    println!("DEBUG calling get_website_metadata_from_network()");
    let metadata = match get_website_metadata_from_network(xor_name, files_api).await {
        Ok(metadata) => {
            println!("DEBUG got metadata");
            metadata
        }
        Err(err) => {
            let message = format!("Failed to parse XOR address [{:?}]", err);
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes());
        }
    };

    let xor_name = match metadata.lookup_resource(&resource_path) {
        Ok(xor_name) => xor_name,
        Err(status_code) => {
            let message = format!("Resource not found at {resource_path}");
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(status_code)
                .body(message.into_bytes());
        }
    };

    awe_fetch_xor_data(xor_name, files_api).await
}

/// fetch data using just an xor address (xor:// protocol)
async fn handle_protocol_xor(
    req: &Request,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Hello from handle_protocol_xor()");

    // TODO test if need to handle trailing slash
    let autonomi_url = String::from(req.uri());
    let xor_name = match awe_client::str_to_xor_name(&autonomi_url) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address [{:?}]", err);
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes());
        }
    };

    return awe_fetch_xor_data(xor_name, files_api).await;
}

/// Fetch data from network and return as an http Response
async fn awe_fetch_xor_data(
    xor_name: XorName,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Fetching xor data: {xor_name:64x}");

    match awe_client::autonomi_get_file(xor_name, files_api).await {
        Ok(content) => {
            println!("Retrieved {} bytes", content.len());
            return tauri::http::ResponseBuilder::new().body(content.to_vec());
        }
        Err(e) => {
            let (status, status_message) = tauri_http_status_from_network_error(&e);
            let error_message = format!("{:?}", e);
            let body_message =
                format!("Failed to retrieve data at [{xor_name:64x}]: {error_message}");
            println!("{body_message}\n{status_message}");

            return tauri::http::ResponseBuilder::new()
                .status(status)
                .body(body_message.into_bytes());
        }
    }
}

// TODO Improve autonomi application level API errors (e.g. in a crate, or in the Autonomi APIs).
// TODO Autonomi API errors are largely internal. Could do with a subset of API errors for apps.
// The following are a very selective sample
pub fn tauri_http_status_from_network_error(error: &sn_client::Error) -> (StatusCode, String) {
    let message: String;

    match error {
        // GetRecordError(GetRecordError(Deserialization)) => (
        //     StatusCode::INTERNAL_SERVER_ERROR,
        //     "Internal Server Error - deserialisation failed",
        // ),
        sn_client::Error::Deserialization(error) => {
            message = format!("Internal Server Error - deserialisation failed ({error})");
            (StatusCode::INTERNAL_SERVER_ERROR, message.clone())
        }
        sn_client::Error::Network(NetworkError::GetRecordError(GetRecordError::RecordNotFound)) => {
            (StatusCode::NOT_FOUND, String::from("404 Not found"))
        }

        sn_client::Error::Network(_network_error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            String::from("Unknown error (NetworkError))"),
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            String::from("Unknown error (or default)"),
        ),
    }
}
