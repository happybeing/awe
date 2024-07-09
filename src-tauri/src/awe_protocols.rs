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

use http::{status::StatusCode, Request};

use sn_client::{
    networking::{GetRecordError, NetworkError},
    FilesApi,
};
use sn_registers::RegisterAddress;

use crate::awe_client;
use crate::awe_website_versions::lookup_resource_for_website_version;

use std::sync::Mutex;
lazy_static::lazy_static! {
    pub static ref STATIC_CLI_URL: Mutex<String> =
        Mutex::<String>::new(String::from(""));

    // Set true by UI when it is about to navigate by setting the source URI
    static ref STATIC_NEXT_LOAD_IS_ADDRESS_BAR: Mutex<bool> = Mutex::<bool>::new(true);

    // Set true by UI after a page loads, so we can know the next load is not a page resource, but is a new page
    static ref STATIC_SAVE_NEXT_ADDRESS: Mutex<bool> = Mutex::<bool>::new(true);

    // Address of the last site loaded via address bar or user clicking a link
    static ref STATIC_LAST_SITE_ADDRESS: Mutex<String> = Mutex::<String>::new(String::from(""));

    static ref STATIC_VERSION_REQUESTED: Mutex<u64> = Mutex::<u64>::new(0);
    static ref STATIC_VERSION_LOADED: Mutex<u64> = Mutex::<u64>::new(0);
    static ref STATIC_VERSION_MAX: Mutex<u64> = Mutex::<u64>::new(0);
}

pub fn get_next_load_is_address_bar() -> bool {
    let flag = *STATIC_NEXT_LOAD_IS_ADDRESS_BAR.lock().unwrap();
    println!("get_next_load_is_address_bar() returning {}", flag);
    flag
}

pub fn get_save_next_site_address() -> bool {
    let flag = *STATIC_SAVE_NEXT_ADDRESS.lock().unwrap();
    println!("get_save_next_site_address() returning {}", flag);
    flag
}

pub fn get_last_site_address() -> String {
    let site_address = STATIC_LAST_SITE_ADDRESS.lock().unwrap();
    println!("get_last_site_address() returning {}", site_address);
    site_address.clone()
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

pub fn set_next_load_is_address_bar(flag: bool) {
    println!("set_next_load_is_address_bar() set to {}", flag);
    *STATIC_NEXT_LOAD_IS_ADDRESS_BAR.lock().unwrap() = flag;
}

pub fn set_save_next_site_address(flag: bool) {
    println!("set_save_next_site_address() set to {}", flag);
    *STATIC_SAVE_NEXT_ADDRESS.lock().unwrap() = flag;
}

pub fn set_last_site_address(site_address: &String) {
    if get_save_next_site_address() {
        set_save_next_site_address(false);
        println!("set_last_site_address() set to {}", site_address);
        *STATIC_LAST_SITE_ADDRESS.lock().unwrap() = site_address.clone();
    }
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

const PROTOCOL_AWM: &str = "awm://";

//// JavaScript interface

#[cfg(not(feature = "local-discovery"))]
fn is_local_discovery() -> bool {
    false
}

#[cfg(feature = "local-discovery")]
fn is_local_discovery() -> bool {
    true
}

#[tauri::command]
fn on_set_save_next_site_address(flag: bool) {
    println!("TTTTTTTT on_set_save_next_site_address() setting save_next_address: {flag}");
    set_save_next_site_address(flag);
}

// Obtain any URL provided to the CLI
#[tauri::command]
fn on_get_last_site_address() -> String {
    let last_site_address = get_last_site_address();

    println!("TTTTTTTT tauri::cmd on_get_last_site_address() returning: {last_site_address}");
    last_site_address
}

#[tauri::command]
fn on_is_local_network() -> bool {
    let is_local_network = is_local_discovery();

    println!("TTTTTTTT tauri::cmd on_is_local_network() returning: {is_local_network}");
    is_local_network
}

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
// Returns value truncated to the range 0..max version) or ZERO if no website loaded
#[tauri::command]
fn on_prep_to_load_from_address_bar(frontend_version: usize) -> usize {
    let mut version = frontend_version as u64;

    let max_version = get_version_max();
    if max_version > 0 {
        if version > max_version {
            version = max_version;
        }
    }
    set_next_load_is_address_bar(true);

    println!("TTTTTTTT on_prep_to_load_from_address_bar({frontend_version}) returning version: {version}");
    set_version_requested(version);
    version as usize
}

use color_eyre::eyre::{eyre, Report};
use std::collections::HashMap;
use url::Url;
const PROTOCOL_END_STR: &str = "://";
const URL_PARAM_VERSION: &str = "v";

/// Parse an awe compatible URL into a tuple of:
///   String protocol (e.g. "awv://")
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
//#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn register_protocols(cli_url: Option<String>, cli_website_version: Option<u64>) {
    if cli_url.is_some() {
        *STATIC_CLI_URL.lock().unwrap() = cli_url.unwrap().clone();
    };

    println!("CLI specified --website-version {:?}", cli_website_version);
    if cli_website_version.is_some() {
        set_version_requested(cli_website_version.unwrap());
    };

    tauri::Builder::default()
        // Rust functions available to JavaScript
        .invoke_handler(tauri::generate_handler![
            on_set_save_next_site_address,
            on_get_last_site_address,
            on_is_local_network,
            on_start_get_cli_url,
            on_prep_to_load_from_address_bar,
            on_get_version_loaded,
            on_get_version_requested,
            on_get_version_max,
        ])
        .register_uri_scheme_protocol("test", |_app, req| {
            let url = req.uri();
            let content =
                format!("<HTML><HEAD></HEAD><BODY><h1>{url:?}</h1> content</BODY></HTML>");
            http::Response::builder()
                .header("Content-Type", "text/html")
                .body(content.into_bytes())
                .unwrap()
        })
        // Protocol for a file
        .register_uri_scheme_protocol("awf", move |_app, req| {
            tauri::async_runtime::block_on(async move { handle_protocol_awf(&req).await })
        })
        // Protocol for a website (WebsiteMetadata)
        .register_uri_scheme_protocol("awm", move |_app, req| {
            tauri::async_runtime::block_on(async move { handle_protocol_awm(&req).await })
        })
        // Protocol for a versioned website (WebsiteVersions)
        .register_uri_scheme_protocol("awv", move |_app, req| {
            let website_version = Some(get_version_requested());
            tauri::async_runtime::block_on(async move {
                handle_protocol_awv(&req, website_version).await
            })
        })
        .register_uri_scheme_protocol("awe", move |_app, req| {
            let website_version = Some(get_version_requested());
            tauri::async_runtime::block_on(async move {
                handle_protocol_awe(&req, website_version).await
            })
        })
        // The following macro may give the following 'cargo check' error which can be ignored.
        //      `frontendDist` configuration is set to `"../build"` but this path
        // More here: https://github.com/tauri-apps/tauri/issues/3142
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
    req: &Request<Vec<u8>>,
    version_requested: Option<u64>,
) -> http::Response<Vec<u8>> {
    println!("DEBUG Hello from handle_protocol_awe() version_requested {version_requested:?}");
    let url = req.uri();
    let content =
        format!("<HTML><HEAD></HEAD><BODY><h1>Handling Autonomi Request</h1>{url:?}</BODY></HTML>");
    http::Response::builder()
        .body(content.into_bytes())
        .expect("http::Response::builder() failed")
}

/// Fetch using xor URL for website versions (awv://)
/// Returns content as an http Response
async fn handle_protocol_awv(
    req: &Request<Vec<u8>>,
    version_requested: Option<u64>,
) -> http::Response<Vec<u8>> {
    println!("DEBUG Hello from handle_protocol_awv() version_requested {version_requested:?}");
    let url = req.uri();
    println!("DEBUG url '{url}'");

    let (_protocol, host_xor_string, resource_path, url_params) =
        match parse_url_string(req.uri().to_string()) {
            Ok((_protocol, host_xor_string, resource_path, url_params)) => {
                (_protocol, host_xor_string, resource_path, url_params)
            }
            Err(e) => {
                return http::Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(format!("Failed to parse URL, error: {:?}", e).into_bytes())
                    .unwrap()
            }
        };

    // Loading URI via address bar:
    let loading_new_page_via_address_bar = get_next_load_is_address_bar();
    set_next_load_is_address_bar(false);

    // Loading due to click on link, or in page JavaScript:
    let loading_new_page_via_page =
        get_save_next_site_address() && !loading_new_page_via_address_bar;

    // Loading into current page:
    let loading_resource = !loading_new_page_via_address_bar && !loading_new_page_via_page;

    let mut xor_host_differs_from_page = false;
    let last_site_address = get_last_site_address();
    if let Some(position) = last_site_address.find(host_xor_string.as_str()) {
        if position != "awv://".len() {
            xor_host_differs_from_page = true;
        };
    } else {
        xor_host_differs_from_page = true;
    }

    let mut website_version = version_requested;

    println!("DEBUG loading_new_page_via_address_bar: {loading_new_page_via_address_bar}");
    println!("DEBUG loading_new_page_via_page       : {loading_new_page_via_page}");
    println!("DEBUG loading_resource                : {loading_resource}");
    println!("DEBUG xor_host_differs_from_page      : {xor_host_differs_from_page}");
    println!("DEBUG version_requested               : {version_requested:?}");

    // If the URL specifies a version use that instead
    if let Some(param_version) = url_params.get(URL_PARAM_VERSION) {
        match param_version.parse() {
            Ok(version_number) => website_version = Some(version_number),
            Err(_e) => {
                println!("DEBUG number expected for URL parameter '{URL_PARAM_VERSION}'='{param_version}'")
            }
        }
    }

    if loading_resource && !xor_host_differs_from_page && website_version.is_none() {
        if get_version_loaded() > 0 {
            website_version = Some(get_version_loaded());
        }
    }

    if loading_new_page_via_page && !xor_host_differs_from_page && website_version.is_none() {
        if get_version_loaded() > 0 {
            website_version = Some(get_version_loaded());
        }
    }

    println!("DEBUG (host_xor_string, resource_path): ({host_xor_string}, {resource_path})'");
    let versions_register_address = match RegisterAddress::from_hex(&host_xor_string.as_str()) {
        Ok(versions_register_address) => versions_register_address,
        Err(err) => {
            let message = format!("Failed to parse RegisterAddress address [{:?}]", err);
            println!("{message}");
            return http::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    // Initialise network connection, client and files api
    let files_api = awe_client::connect_to_autonomi()
        .await
        .expect("Failed to connect to Autonomi Network");

    // Save in case we don't want site version changed
    let current_site_version = get_version_loaded();

    let xor_name = match lookup_resource_for_website_version(
        &resource_path,
        versions_register_address,
        website_version,
        &files_api,
    )
    .await
    {
        Ok(xor_name) => xor_name,
        Err(status_code) => {
            let message = format!("Resource not found at {resource_path}");
            println!("{message}");
            return http::Response::builder()
                .status(status_code)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    let response = awe_fetch_xor_data(xor_name, Some(&files_api)).await;
    if response.status() == StatusCode::OK {
        // Keep site version unchanged when loading a resource
        if loading_resource {
            set_version_loaded(current_site_version);
        }

        // After loading a new page update the site address
        if loading_new_page_via_address_bar
            || loading_new_page_via_page && xor_host_differs_from_page
        {
            set_last_site_address(&url.to_string());
        }
    }
    response
}

/// Fetch using an xor URL for a website (WebsiteMetadata) (awm://)
/// Returns content as an http Response
async fn handle_protocol_awm(req: &Request<Vec<u8>>) -> http::Response<Vec<u8>> {
    println!("DEBUG Hello from handle_protocol_awm()");

    let url = req.uri().to_string();
    println!("DEBUG url '{url}'");
    let (_, remainder) = if url.starts_with(PROTOCOL_AWM) {
        url.split_at(PROTOCOL_AWM.len())
    } else {
        ("", url.as_str())
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
            return http::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    // Initialise network connection, client and files api
    let files_api = awe_client::connect_to_autonomi()
        .await
        .expect("Failed to connect to Autonomi Network");

    println!("DEBUG calling get_website_metadata_from_network()");
    let metadata = match get_website_metadata_from_network(xor_name, &files_api).await {
        Ok(metadata) => {
            println!("DEBUG got metadata");
            metadata
        }
        Err(err) => {
            let message = format!("Failed to parse XOR address. [{:?}]", err);
            println!("{message}");
            return http::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    let xor_name = match metadata.lookup_resource(&resource_path) {
        Ok(xor_name) => xor_name,
        Err(status_code) => {
            let message = format!("Resource not found at {resource_path}");
            println!("{message}");
            return http::Response::builder()
                .status(status_code)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    let response = awe_fetch_xor_data(xor_name, Some(&files_api)).await;
    if response.status() == StatusCode::OK {
        set_last_site_address(&url.to_string());
    }
    response
}

/// Fetch a file using just an xor address (awf://)
/// Returns content as an http Response
async fn handle_protocol_awf(req: &Request<Vec<u8>>) -> http::Response<Vec<u8>> {
    println!("DEBUG Hello from handle_protocol_awf()");

    // Initialise network connection, client and files api
    let files_api = awe_client::connect_to_autonomi()
        .await
        .expect("Failed to connect to Autonomi Network");

    // TODO test if need to handle trailing slash
    let autonomi_url = req.uri().to_string();
    let xor_name = match awe_client::str_to_xor_name(&autonomi_url.as_str()) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address. [{:?}]", err);
            println!("{message}");
            return http::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes())
                .unwrap();
        }
    };

    return awe_fetch_xor_data(xor_name, Some(&files_api)).await;
}

/// Fetch data from network and return as an http Response
async fn awe_fetch_xor_data(
    xor_name: XorName,
    files_api_opt: Option<&FilesApi>,
) -> http::Response<Vec<u8>> {
    println!("Fetching xor data: {xor_name:64x}");

    // Initialise network connection, client and files api
    let api;
    let files_api_ref;
    if let Some(api_ref) = files_api_opt {
        files_api_ref = api_ref;
    } else {
        api = awe_client::connect_to_autonomi()
            .await
            .expect("Failed to connect to Autonomi Network");
        files_api_ref = &api;
    }

    // TODO since Tauri v2, the iframe won't load content from
    // TODO a URI unless the response has a Content-Type header
    // TODO Investigate options, such as saving content type in the site map
    match awe_client::autonomi_get_file(xor_name, files_api_ref).await {
        Ok(content) => {
            println!("Retrieved {} bytes", content.len());
            // println!("THIS: {:?} bytes", content);
            return http::Response::builder()
                .header(http::header::CONTENT_TYPE, "text/html") // TODO needed since Tauri switched to using http::Response from tauri::http::ResponseBuilder
                .body(content.to_vec())
                .unwrap();
        }
        Err(e) => {
            let (status, status_message) = tauri_http_status_from_network_error(&e);
            let error_message = format!("{:?}", e);
            let body_message =
                format!("Failed to retrieve data at [{xor_name:64x}]: {error_message}");
            println!("{body_message}\n{status_message}");

            return http::Response::builder()
                .status(status)
                .body(body_message.into_bytes())
                .unwrap();
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
