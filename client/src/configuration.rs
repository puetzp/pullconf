use crate::resources::{
    Resource, {Error, Resources},
};
use common::Hostname;
use log::{debug, info};
use std::{
    collections::{HashMap, VecDeque},
    env, fs,
    io::ErrorKind,
    process::Command,
    str::FromStr,
    time::{Instant, SystemTime},
};
use ureq::{tls, Agent};
use url::Url;

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub pid: u32,
    pub timestamp_ms: usize,
    pub duration_ms: usize,
    pub resources: Vec<Resource>,
}

const ETAG_FILE: &str = "/var/lib/pullconf/etag";
const DATA_FILE: &str = "/var/lib/pullconf/data";

/// This struct contains every piece of information that is needed to retrieve
/// this system's configuration (resource list) from pullconfd and apply it.
#[derive(Debug)]
pub struct Configuration {
    agent: Agent,
    base_url: Url,
    api_key: String,
    resources: VecDeque<Resource>,
}

impl Configuration {
    /// Retrieve this system's configuration from pullconfd.
    /// Depending on pullconfd's answer, either the payload or the cached resource
    /// list are parsed from JSON and then returned.
    pub fn get() -> Result<Self, String> {
        // Retrieve the system's (fully-qualified) hostname. The hostname is used
        // to query pullconfd for this system's configuration.
        let hostname = {
            let mut command = Command::new("hostname");
            command.arg("--fqdn");

            let result = command
                .output()
                .map_err(|error| format!("failed to execute {:?}: {}", command, error))?;

            if result.status.success() {
                let output = String::from_utf8(result.stdout).map_err(|error| {
                    format!("failed to read stdout from {:?}: {}", command, error)
                })?;

                Hostname::from_str(output.as_str().trim()).map_err(|error| {
                    format!("failed to parse output from {:?}: {}", command, error)
                })?
            } else {
                return Err(format!(
                    "failed to execute {:?}, returned non-zero exit code",
                    command
                ));
            }
        };

        let base_url = {
            let address = {
                let v = "PULLCONF_SERVER";
                match env::var(v) {
                    Ok(value) => format!("https://{}", value),
                    Err(error) => {
                        return Err(format!(
                            "failed to read environment variable `{}`: {}",
                            v, error
                        ))
                    }
                }
            };

            Url::parse(&address)
                .map_err(|error| format!("failed to parse `{}` as URL: {}", address, error))?
        };

        // The API key that is defined in the TOML configuration file on the server.
        let api_key = {
            let v = "PULLCONF_API_KEY";

            env::var(v).map_err(|error| {
                format!("failed to read environment variable `{}`: {}", v, error)
            })?
        };

        // Initialize the default crypto provider.
        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::aws_lc_rs::default_provider(),
        );

        // Build a TLS configuration from trusted roots.
        // Custom Certificate Authorities should be added to the
        // platform's trust store in order for them to function
        // properly.
        let tls_config = tls::TlsConfig::builder()
            .root_certs(tls::RootCerts::PlatformVerifier)
            .build();

        // Initialize the agent used to communicate with pullconfd.
        let config = Agent::config_builder()
            .https_only(true)
            .tls_config(tls_config)
            .http_status_as_error(false)
            .build();

        let agent: Agent = config.into();

        // Both successful and erroneous responses from pullconfd are JSON. Except when
        // the response comes from an intermediary (e.g. a reverse proxy).
        let content_type = "application/json";

        // Query pullconfd for this system's configuration and parse the result.
        let url = base_url
            .join(&format!("/api/clients/{}/resources", hostname))
            .unwrap();

        let mut request = agent
            .get(url.as_str())
            .header("accept", content_type)
            .header("x-api-key", &api_key);

        debug!("checking if a file with an etag of a saved resource list exists",);

        if let Some(etag) = get_etag()? {
            debug!("adding etag of saved resource list to request",);
            request = request.header("if-none-match", &etag);
        }

        let _timer = Instant::now();

        debug!("requesting resource list from `{}`", url);

        let resources = match request.call().inspect(|response| {
            if let Some(content_length) = response
                .headers()
                .get("content-length")
                .and_then(|value| value.to_str().ok())
            {
                debug!("received {} bytes", content_length);
            }

            debug!(
                "finished request in {} ms",
                (_timer.elapsed().as_millis() as f64) / 1000.0
            )
        }) {
            Ok(mut response) => {
                if response.status() == 304 {
                    debug!("server returned 304, ignoring the request body and reading saved resource list from disk");

                    get_saved_resource_list()?.data
                } else if response.status().is_client_error() || response.status().is_server_error()
                {
                    // If the response is erroneous according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if let Some(_content_type) = response
                        .body()
                        .mime_type()
                        .filter(|value| *value != content_type)
                    {
                        return Err(format!(
                            "unexpected API response content type, expected `{}`, got `{}` and status `{}` from `{}`",
                            content_type,
                            _content_type,
                            response.status(),
                            response.headers().get("server").and_then(|value| value.to_str().ok()).unwrap_or_default()
                        ));
                    } else {
                        debug!(
                            "content type is `{}`, deserializing error message",
                            content_type
                        );

                        // Otherwise parse the well-known API error format from JSON and log
                        // the error appropiately. Then terminate the program.
                        let error = response.body_mut().read_json::<Error>().map_err(|error| {
                            format!("failed to deserialize error response: {}", error)
                        })?;

                        return Err(format!(
                            "pullconfd failed to process the request: {}, {}",
                            error.title, error.detail
                        ));
                    }
                } else {
                    // If the response is successful according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if let Some(_content_type) = response
                        .body()
                        .mime_type()
                        .filter(|value| *value != content_type)
                    {
                        return Err(format!(
                            "unexpected API response content type, expected `{}`, got `{}` and status `{}` from `{}`",

                            content_type,
                            _content_type,
                            response.status(),
                            response.headers().get("server").and_then(|value| value.to_str().ok()).unwrap_or_default()
                        ));
                    } else {
                        let etag = response
                            .headers()
                            .get("etag")
                            .and_then(|value| value.to_str().ok().map(|value| value.to_string()));

                        debug!(
                            "content type is `{}`, deserializing resource list",
                            content_type
                        );

                        // Otherwise parse the payload as it is expected to be a JSON-encoded
                        // resource list.
                        let payload = response.body_mut().read_to_string().map_err(|error| {
                            format!("failed to parse payload as utf-8 string: {}", error)
                        })?;

                        if let Some(etag) = etag {
                            debug!("saving resource list to disk");

                            save_resource_list(&etag, &payload)?;
                        }

                        serde_json::from_str::<Resources>(&payload)
                            .map_err(|error| {
                                format!("failed to deserialize resource list : {}", error)
                            })?
                            .data
                    }
                }
            }
            Err(error) => {
                // Log any unexpected errors as-is and terminate the program.
                return Err(format!("failed to send request to pullconfd: {}", error));
            }
        };

        let configuration = Self {
            agent,
            base_url,
            api_key,
            resources,
        };

        Ok(configuration)
    }

    /// Apply every resource that is part of this system's configuration.
    /// Resources are applied in no particular order. Every resource
    /// checks if it has any dependencies and if those were alreay applied.
    /// If not, move on to the next resource. If the resource is ready, apply
    /// it.
    /// Since there are always resources that have no dependencies, those are
    /// applied first and then everything else, until every resource has been
    /// applied.
    pub fn apply(mut self, pid: u32) -> Report {
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as usize;

        let timer = Instant::now();

        let mut applied_resources = HashMap::with_capacity(self.resources.len());

        let mut order = 0;

        while let Some(mut resource) = self.resources.pop_front() {
            if !resource.is_ready(&applied_resources) {
                self.resources.push_back(resource);
                continue;
            }

            resource.apply(
                order,
                &self.agent,
                &self.base_url,
                &self.api_key,
                &applied_resources,
            );

            order = order + 1;

            applied_resources.insert(resource.id(), resource);
        }

        let mut resources: Vec<Resource> = applied_resources.into_values().collect();

        resources.sort_by(|a, b| a.order().cmp(&b.order()));

        let duration_ms = timer.elapsed().as_millis() as usize;

        info!(
            "applied resource list in {:.3} seconds",
            duration_ms as f64 / 1000.0
        );

        Report {
            pid,
            timestamp_ms,
            duration_ms,
            resources,
        }
    }
}

fn get_etag() -> Result<Option<String>, String> {
    match fs::read_to_string(ETAG_FILE) {
        Ok(etag) => {
            if etag.is_empty() {
                Ok(None)
            } else {
                Ok(Some(etag))
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            debug!("etag file does not exist");
            Ok(None)
        }
        Err(error) => {
            return Err(format!(
                "failed to read etag file `{}`: {}",
                ETAG_FILE, error
            ));
        }
    }
}

fn get_saved_resource_list() -> Result<Resources, String> {
    let content = fs::read_to_string(DATA_FILE).map_err(|error| {
        format!(
            "failed to read resource list from file `{}`: {}",
            DATA_FILE, error
        )
    })?;

    serde_json::from_str::<Resources>(&content).map_err(|error| {
        format!(
            "failed to deserialize resource list from file `{}`: {}",
            DATA_FILE, error
        )
    })
}

fn save_resource_list(etag: &str, data: &str) -> Result<(), String> {
    if let Err(error) = fs::write(ETAG_FILE, etag) {
        return Err(format!(
            "failed to save latest resource list etag to file `{}`: {}",
            ETAG_FILE, error
        ));
    }

    if let Err(error) = fs::write(DATA_FILE, data) {
        return Err(format!(
            "failed to save latest resource list to file `{}`: {}",
            DATA_FILE, error
        ));
    }

    Ok(())
}
