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
use sn_protocol::version;
use tauri::http::{status::StatusCode, Request, Response};

use sn_client::{
    networking::{GetRecordError, NetworkError},
    Client, FilesApi,
};
use sn_peers_acquisition::get_peers_from_args;
use sn_registers::RegisterAddress;

use crate::awe_client;
use crate::awe_website_versions::lookup_resource_for_website_version;
use crate::cli_options::Opt;

use std::sync::Mutex;
lazy_static::lazy_static! {
    pub static ref STATIC_CLI_URL: Mutex<String> =
        Mutex::<String>::new(String::from(""));

    static ref STATIC_VERSION_REQUESTED: Mutex<u64> = Mutex::<u64>::new(0);
    static ref STATIC_VERSION_LOADED: Mutex<u64> = Mutex::<u64>::new(0);
    static ref STATIC_VERSION_MAX: Mutex<u64> = Mutex::<u64>::new(0);
}

pub fn get_version_requested() -> u64 {
    let version = *STATIC_VERSION_REQUESTED.lock().unwrap();
    println!("get_version_requested() returning {}", version);
    version
}

pub fn get_version_loaded() -> u64 {
    let version = *STATIC_VERSION_LOADED.lock().unwrap();
    println!("get_version_loaded() returning {}", version);
    version
}

pub fn get_version_max() -> u64 {
    let version = *STATIC_VERSION_MAX.lock().unwrap();
    println!("get_version_max() returning {}", version);
    version
}

pub fn set_version_requested(version: u64) {
    println!("set_version_requested() set to {}", version);
    *STATIC_VERSION_REQUESTED.lock().unwrap() = version;
}

pub fn set_version_loaded(version: u64) {
    println!("set_version_loaded() set to {}", version);
    *STATIC_VERSION_LOADED.lock().unwrap() = version;
}
pub fn set_version_max(version: u64) {
    println!("set_version_max() set to {}", version);
    *STATIC_VERSION_MAX.lock().unwrap() = version;
}

const PROTOCOL_AWX: &str = "awx://";
const PROTOCOL_AMX: &str = "amx://";

//// JavaScript interface

// Obtain any URL provided to the CLI
#[tauri::command]
fn on_start_get_cli_url() -> String {
    let cli_url = STATIC_CLI_URL.lock().unwrap();
    println!("TTTTTTTT tauri::cmd on_start_get_cli_url() returning: {cli_url}");
    cli_url.to_string()
}

// Obtain website version requested on the CLI
#[tauri::command]
fn on_get_version_requested() -> usize {
    let version = get_version_requested() as usize;
    println!("TTTTTTTT tauri::cmd on_get_version_requested() returning {version}");
    version as usize
}

// Obtain loaded website version, or the one provided to the CLI
#[tauri::command]
fn on_get_version_loaded() -> usize {
    let version = get_version_loaded() as usize;
    println!("TTTTTTTT tauri::cmd on_get_version_loaded() returning {version}");
    version as usize
}

#[tauri::command]
fn on_get_version_max() -> usize {
    let version = get_version_max() as usize;
    println!("TTTTTTTT tauri::cmd on_get_version_max() called from JS, returning {version}",);
    version
}

// Called by frontend before setting a new version URL parameter
// Returns value truncated to the range 1..max version) or ZERO if no website loaded
#[tauri::command]
fn on_frontend_set_version(frontend_version: usize) -> usize {
    let mut version = frontend_version as u64;

    let max_version = get_version_max();
    if max_version > 0 {
        if version > max_version {
            version = max_version;
        }
    }

    println!("TTTTTTTT on_frontend_set_version({frontend_version}) returning version: {version}");
    set_version_requested(version);
    version as usize
}

use color_eyre::eyre::{eyre, Report};
use std::collections::HashMap;
use url::Url;
const PROTOCOL_END_STR: &str = "://";
const URL_PARAM_VERSION: &str = "v";

/// Parse an awe compatible URL into a tuple of:
///   String protocol (e.g. "awx://")
///   String XOR-ADDRESS or NRS host (including subdomains)
///   String of the path part
///   HashMap of query parameters to values
fn parse_url_string(
    url: String,
) -> Result<(String, String, String, HashMap<String, String>), Report> {
    println!("DEBUG parse_url_string({url}");

    let protocol: String;
    if let Some(colon_position) = url.find(PROTOCOL_END_STR) {
        protocol = url[0..colon_position + PROTOCOL_END_STR.len()].to_string();
        println!("DEBUG   protocol: {protocol}")
    } else {
        return Err(eyre!("Failed to parse URL (missing protocol): {}", url));
    }

    let parsed_url = Url::parse(url.as_str())?;

    if !parsed_url.has_host() {
        return Err(eyre!("Failed to parse URL (no host found): {}", url));
    }
    let query_params: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();

    println!("DEBUG   host: {}", parsed_url.host().unwrap());
    println!("DEBUG   path: {}", parsed_url.path().to_string());
    println!("DEBUG   params: {:?}", query_params);

    Ok((
        protocol,
        String::from(parsed_url.host_str().unwrap()),
        parsed_url.path().to_string(),
        query_params,
    ))
}

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
pub async fn register_protocols(cli_url: Option<String>, cli_website_version: Option<u64>) {
    if cli_url.is_some() {
        *STATIC_CLI_URL.lock().unwrap() = cli_url.unwrap().clone();
    };

    println!("CLI specified --website-version {:?}", cli_website_version);
    if cli_website_version.is_some() {
        set_version_requested(cli_website_version.unwrap());
    };

    tauri::Builder::default()
        // Rust functions available to JavaScript
        // TODO these handlers are flakey. Work, then don't for no obvious reason. Not sure how to debug
        .invoke_handler(tauri::generate_handler![
            on_start_get_cli_url,
            on_frontend_set_version,
            on_get_version_loaded,
            on_get_version_requested,
            on_get_version_max,
        ])
        // Protocol for a file
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
                handle_protocol_xor(&req, &files_api.clone()).await
            })
        })
        // Protocol for a website (WebsiteMetadata)
        .register_uri_scheme_protocol("amx", move |_app, req| {
            set_version_loaded(0);
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
                handle_protocol_amx(&req, &files_api.clone()).await
            })
        })
        // Protocol for a versioned website (WebsiteVersions)
        .register_uri_scheme_protocol("awx", move |_app, req| {
            set_version_loaded(0);
            let website_version = Some(get_version_requested());
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
                handle_protocol_awx(&req, website_version, &client, &files_api.clone()).await
            })
        })
        .register_uri_scheme_protocol("awe", move |_app, req| {
            let website_version = Some(get_version_requested());
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
                handle_protocol_awe(&req, website_version, &files_api.clone()).await
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
/// Fetch using a webname URL for website versions (awe://)
/// Returns content as an http Response
async fn handle_protocol_awe(
    req: &Request,
    website_version: Option<u64>,
    _files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("DEBUG Hello from handle_protocol_awe() website_version {website_version:?}");
    let url = req.uri();
    let content =
        format!("<HTML><HEAD></HEAD><BODY><h1>Handling Autonomi Request</h1>{url:?}</BODY></HTML>");
    tauri::http::ResponseBuilder::new().body(content.into_bytes())
}

/// Fetch using xor URL for website versions (awx://)
/// Returns content as an http Response
async fn handle_protocol_awx(
    req: &Request,
    website_version: Option<u64>,
    client: &Client,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("DEBUG Hello from handle_protocol_awx() website_version {website_version:?}");
    let url = req.uri();
    println!("DEBUG url '{url}'");

    let (_protocol, host_xor_string, resource_path, url_params) =
        parse_url_string(req.uri().to_string())?;

    // If the URL specifies a version use that instead
    let mut website_version = website_version;
    if let Some(param_version) = url_params.get(URL_PARAM_VERSION) {
        match param_version.parse() {
            Ok(version_numner) => website_version = Some(version_numner),
            Err(_e) => {
                println!("DEBUG number expected for URL parameter '{URL_PARAM_VERSION}'='{param_version}'")
            }
        }
    }

    println!("DEBUG (host_xor_string, resource_path): ({host_xor_string}, {resource_path})'");
    let versions_register_address = match RegisterAddress::from_hex(&host_xor_string.as_str()) {
        Ok(versions_register_address) => versions_register_address,
        Err(err) => {
            let message = format!("Failed to parse RegisterAddress address [{:?}]", err);
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes());
        }
    };

    let xor_name = match lookup_resource_for_website_version(
        &resource_path,
        versions_register_address,
        website_version,
        client,
        files_api,
    )
    .await
    {
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

// /// Fetch using xor URL for website versions (awx://)
// /// Returns content as an http Response
// async fn handle_protocol_awx(
//     req: &Request,
//     website_version: Option<u64>,
//     client: &Client,
//     files_api: &FilesApi,
// ) -> Result<Response, Box<dyn std::error::Error>> {
//     println!("DEBUG Hello from handle_protocol_awx() website_version {website_version:?}");
//     let url = req.uri();
//     println!("DEBUG url '{url}'");

//     let (protocol, host, path, params) = parse_url_string(req.uri().to_string())?;

//     let (_, remainder) = if url.starts_with(PROTOCOL_AWX) {
//         url.split_at(PROTOCOL_AWX.len())
//     } else {
//         ("", url)
//     };

//     let mut remainder = remainder.to_string();
//     let (xor_string, resource_path) = match remainder.find(PATH_SEPARATOR) {
//         Some(separator_position) => {
//             let path_part = remainder.split_off(separator_position);
//             (remainder, path_part)
//         }
//         None => (remainder, String::from(PATH_SEPARATOR)),
//     };

//     println!("DEBUG (xor_string, resource_path): ({xor_string}, {resource_path})'");
//     let versions_register_address = match RegisterAddress::from_hex(&xor_string.as_str()) {
//         Ok(versions_register_address) => versions_register_address,
//         Err(err) => {
//             let message = format!("Failed to parse RegisterAddress address [{:?}]", err);
//             println!("{message}");
//             return tauri::http::ResponseBuilder::new()
//                 .status(StatusCode::BAD_REQUEST)
//                 .body(message.into_bytes());
//         }
//     };

//     let xor_name = match lookup_resource_for_website_version(
//         &resource_path,
//         versions_register_address,
//         website_version,
//         client,
//         files_api,
//     )
//     .await
//     {
//         Ok(xor_name) => xor_name,
//         Err(status_code) => {
//             let message = format!("Resource not found at {resource_path}");
//             println!("{message}");
//             return tauri::http::ResponseBuilder::new()
//                 .status(status_code)
//                 .body(message.into_bytes());
//         }
//     };

//     awe_fetch_xor_data(xor_name, files_api).await
// }

/// Fetch using an xor URL for a website (WebsiteMetadata) (amx://)
/// Returns content as an http Response
async fn handle_protocol_amx(
    req: &Request,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("DEBUG Hello from handle_protocol_amx()");

    let url = req.uri();
    println!("DEBUG url '{url}'");
    let (_, remainder) = if url.starts_with(PROTOCOL_AMX) {
        url.split_at(PROTOCOL_AMX.len())
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
    let xor_name = match awe_client::str_to_xor_name(&xor_string.as_str()) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address. [{:?}]", err);
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
            let message = format!("Failed to parse XOR address. [{:?}]", err);
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

/// Fetch a file using just an xor address (xor://)
/// Returns content as an http Response
async fn handle_protocol_xor(
    req: &Request,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("DEBUG Hello from handle_protocol_xor()");

    // TODO test if need to handle trailing slash
    let autonomi_url = String::from(req.uri());
    let xor_name = match awe_client::str_to_xor_name(&autonomi_url.as_str()) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address. [{:?}]", err);
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
