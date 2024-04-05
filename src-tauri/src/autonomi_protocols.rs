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

use sn_client::{
    networking::{GetRecordError, NetworkError},
    Client, FilesApi,
};
// use std::error::Error;
use tauri::http::{status::StatusCode, Request, Response};

use crate::autonomi_client;

pub async fn handle_protocol_axor(
    _client: &Client,
    req: &Request,
    files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Hello from handle_protocol_axor()");

    let autonomi_url = String::from(req.uri());
    // let content = format!(
    //     "<HTML><HEAD></HEAD><BODY><h1>Handling Autonomi Request</h1>{autonomi_url:?}</BODY></HTML>"
    // );

    let xor_name = match autonomi_client::str_to_xor_name(&autonomi_url) {
        Ok(xor_name) => xor_name,
        Err(err) => {
            let message = format!("Failed to parse XOR address [{:?}]", err);
            println!("{message}");
            return tauri::http::ResponseBuilder::new()
                .status(StatusCode::BAD_REQUEST)
                .body(message.into_bytes());
        }
    };

    match autonomi_client::autonomi_get_file(xor_name, files_api).await {
        Ok(content) => {
            println!("Successfully retrieved data at [{}]", autonomi_url);
            return tauri::http::ResponseBuilder::new().body(content.to_vec());
        }
        Err(e) => {
            // TODO Improve autonomi application level API errors (e.g. in a crate, or in the Autonomi APIs).
            // TODO Autonomi API errors are largely internal. Could do with a subset of API errors for apps.
            // The following are a very selective sample
            let message: String;
            let error_message = format!("{:?}", e);
            let (status, status_message) = match e {
                // GetRecordError(GetRecordError(Deserialization)) => (
                //     StatusCode::INTERNAL_SERVER_ERROR,
                //     "Internal Server Error - deserialisation failed",
                // ),
                sn_client::Error::Deserialization(error) => {
                    message = format!("Internal Server Error - deserialisation failed ({error})");
                    (StatusCode::INTERNAL_SERVER_ERROR, message.as_str())
                }
                sn_client::Error::Network(NetworkError::GetRecordError(
                    GetRecordError::RecordNotFound,
                )) => (StatusCode::NOT_FOUND, "404 Not found"),

                sn_client::Error::Network(_network_error) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Unknown error (NetworkError)",
                ),
                _ => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Unknown error (or default)",
                ),
            };

            let body_message =
                format!("Failed to retrieve data at [{autonomi_url}]: {error_message}");
            println!("{body_message}\n{status_message}");

            return tauri::http::ResponseBuilder::new()
                .status(status)
                .body(body_message.into_bytes());
        }
    }
}

pub async fn handle_protocol_axweb(
    _client: &Client,
    req: &Request,
    _files_api: &FilesApi,
) -> Result<Response, Box<dyn std::error::Error>> {
    println!("Hello from handle_protocol_axweb()");
    let url = req.uri();
    let content =
        format!("<HTML><HEAD></HEAD><BODY><h1>Handling Autonomi Request</h1>{url:?}</BODY></HTML>");
    tauri::http::ResponseBuilder::new().body(content.into_bytes())
}
