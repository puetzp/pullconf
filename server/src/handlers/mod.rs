pub mod error;

use crate::{
    handlers::error::Error,
    types::{resources::Resource, ApiKey},
    SharedAppState,
};
use common::{Hostname, Links};
use log::debug;
use rand::{distributions::Alphanumeric, Rng};
use rouille::{content_encoding, router, Request, Response};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, time::Instant};

pub fn handle_request(request: &Request, state: SharedAppState) -> Response {
    let start = Instant::now();

    let scope = "api";

    let request_id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>();

    debug!(
        scope,
        request_id,
        url = request.url();
        "received {:?}",
        request
    );

    let header = "x-api-key";

    let response = match request.header(header) {
        Some(key) => {
            debug!(
                scope,
                request_id,
                url = request.url();
                "found {} header",
                header
            );

            match handle_route(&request_id, request, state, key) {
                Ok(r) => r,
                Err(e) => e.into(),
            }
        }
        None => {
            debug!(
                scope,
                request_id,
                url = request.url();
                "client failed to provide authentication credentials via the {} header",
                header,
            );

            Error::missing_authorization().into()
        }
    };

    debug!(
        scope,
        request_id,
        url = request.url();
        "applying optional encoding based on the accept-encoding header",
    );

    let response = content_encoding::apply(request, response);

    debug!(
        scope,
        request_id,
        url = request.url();
        "returning {:?}",
        response
    );

    debug!(
        scope,
        request_id,
        url = request.url();
        "took {} ms to process the request",
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
    let scope = "api";

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
            debug!(
                scope,
                request_id,
                url = request.url();
                "client failed to authenticate"
            );
            return Err(Error::failed_authorization());
        }
    };

    debug!(
        scope,
        request_id,
        url = request.url(),
        client:% = client.name();
        "client authenticated successfully"
    );

    if let Some(request) = request.remove_prefix("/assets") {
        if !client
            .resources
            .files
            .iter()
            .filter_map(|file| {
                file.parameters
                    .source
                    .as_ref()
                    .and_then(|path| path.to_str())
            })
            .any(|path| path == request.url())
        {
            debug!(
                scope,
                request_id,
                url = request.url(),
                client:% = client.name();
                "client is not permitted to download file as none of its associated file resources specify this download path",
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
                            scope,
                            request_id,
                            url = request.url(),
                            client:% = client.name();
                            "client is not permitted to download this resource catalog",
                        );

                        return Ok(Error::forbidden().into());
                    }

                    let mut data: Vec<Resource> = vec![];

                    data.extend(client.resources.directories.iter().map(|item| item.into()));
                    data.extend(client.resources.files.iter().map(|item| item.into()));
                    data.extend(client.resources.groups.iter().map(|item| item.into()));
                    data.extend(client.resources.hosts.iter().map(|item| item.into()));
                    data.extend(client.resources.symlinks.iter().map(|item| item.into()));
                    data.extend(client.resources.users.iter().map(|item| item.into()));
                    data.extend(client.resources.apt_packages.iter().map(|item| item.into()));

                    if let Some(resolv_conf) = &client.resources.resolv_conf {
                        data.push(resolv_conf.into());
                    }

                    let response = ApiResponse {
                        links: Links {
                            this: format!("/api/clients/{}", client.name()),
                            ..Default::default()
                        },
                        data,
                    };

                    let bytes = serde_json::to_vec(&response).unwrap();

                    let etag = format!("{:x}", Sha256::digest(&bytes));

                    Ok(Response::from_data("application/json", bytes).with_etag(request, etag))
                },
                _ => {
                    debug!(
                        scope,
                        request_id,
                        url = request.url(),
                        client:% = client.name();
                        "failed to find route matching this request"
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
