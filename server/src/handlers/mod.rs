pub mod error;

use crate::{handlers::error::Error, types::ApiKey, SharedAppState};
use common::{Hostname, Links};
use log::debug;
use rand::distr::{Alphanumeric, SampleString};
use rouille::{content_encoding, router, Request, Response};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, time::Instant};

pub fn handle_request(request: &Request, state: SharedAppState) -> Response {
    let start = Instant::now();

    let request_id = Alphanumeric.sample_string(&mut rand::rng(), 6);

    debug!("(request: {}) received {:?}", request_id, request);

    let header = "x-api-key";

    let response = match request.header(header) {
        Some(key) => {
            debug!("(request: {}) found {} header", request_id, header);

            match handle_route(&request_id, request, state, key) {
                Ok(r) => r,
                Err(e) => e.into(),
            }
        }
        None => {
            debug!(
                "(request: {}) client failed to provide authentication credentials via the {} header",
                request_id,
                header,
            );

            Error::missing_authorization().into()
        }
    };

    debug!(
        "(request: {}) applying optional encoding based on the accept-encoding header",
        request_id
    );

    let response = content_encoding::apply(request, response);

    debug!("(request: {}) returning {:?}", request_id, response);

    debug!(
        "(request: {}) took {} ms to process the request",
        request_id,
        start.elapsed().as_millis()
    );

    response
}

fn handle_route(
    request_id: &str,
    request: &Request,
    state: SharedAppState,
    api_key: &str,
) -> Result<Response, Error> {
    let state = state.read().unwrap();

    let encrypted_key = ApiKey::encrypt(api_key);

    let client = match state
        .configuration
        .api_keys
        .get(&encrypted_key)
        .and_then(|name| state.configuration.clients.get(name))
    {
        Some(client) => client.clone(),
        None => {
            debug!("(request: {}) client failed to authenticate", request_id,);
            return Err(Error::failed_authorization());
        }
    };

    debug!(
        "(request: {}) client authenticated successfully as `{}`",
        request_id,
        client.name()
    );

    if let Some(request) = request.remove_prefix("/assets") {
        if !client
            .resources
            .iter()
            .filter_map(|resource| {
                resource.as_file().and_then(|file| {
                    file.parameters
                        .source
                        .as_ref()
                        .and_then(|path| path.to_str())
                })
            })
            .any(|path| path == request.url())
        {
            debug!(
                "(request: {}) client `{}` is not permitted to download file as none of its associated file resources specify this download path",
                request_id,
                client.name()
            );

            return Err(Error::forbidden());
        }

        Ok(match_assets(&request, state.assets.clone()))
    } else {
        router!(request,
                (GET) (/api/clients/{hostname: Hostname}/resources) => {
                    // TODO: Since the resource configuration remains unchanged
                    // once the server has loaded, it could be worthwhile to
                    // serialize the whole catalog (per client) once after
                    // validating the configuration, and then serve the
                    // serialized catalog from memory, instead of serializing
                    // the catalog on every request.
                    #[derive(Serialize)]
                    struct ApiResponse<T> {
                        pub links: Links,
                        pub data: T,
                    }

                    if client.name() != &hostname {
                        debug!(
                            "(request: {}) client `{}` is not permitted to download this resource catalog",
                            request_id,
                            client.name()
                        );

                        return Ok(Error::forbidden().into());
                    }

                    let response = ApiResponse {
                        links: Links {
                            this: format!("/api/clients/{}", client.name()),
                            ..Default::default()
                        },
                        data: &client.resources,
                    };

                    let bytes = serde_json::to_vec(&response).unwrap();

                    let etag = format!("{:x}", Sha256::digest(&bytes));

                    Ok(Response::from_data("application/json", bytes).with_etag(request, etag))
                },
                _ => {
                    debug!(
                        "(request: {}) failed to find route matching this request", request_id
                    );

                    Ok(Response::empty_404())
                }
        )
    }
}

fn match_assets(request: &Request, asset_path: PathBuf) -> Response {
    let mut path = asset_path.clone();

    for component in request.url().split('/') {
        path.push(component);
    }

    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Response::empty_404(),
    };

    if !path.starts_with(asset_path) {
        return Response::empty_404();
    }

    if !fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
        return Response::empty_404();
    }

    let mut file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Response::empty_404(),
    };

    let mut bytes = vec![];

    if file.read_to_end(&mut bytes).is_err() {
        return Response::empty_404();
    }

    let etag = format!("{:x}", Sha256::digest(&bytes));

    Response::from_data("application/octet-stream", bytes).with_etag(request, etag)
}
