use crate::resources::{
    Resource, {Error, Resources},
};
use common::{error::Terminate, Hostname};
use log::{debug, error, info};
use std::{
    collections::{HashMap, VecDeque},
    env,
    error::Error as StdError,
    fs,
    io::{BufReader, ErrorKind},
    path::PathBuf,
    process::Command,
    str::FromStr,
    time::Instant,
};
use ureq::{serde_json, Agent, AgentBuilder};
use url::Url;

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
    pub fn get(pid: u32) -> Result<Self, String> {
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

        // Add common CA certificates to the truststore of this request.
        let mut roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        // If a custom directory path is provided that contains other (e.g. self-signed)
        // CA certificates, parse every certificate in each file and add them to
        // the truststore as well.
        if let Ok(ca_dir) = env::var("PULLCONF_CA_DIR") {
            let path = PathBuf::from_str(&ca_dir).map_err(|error| {
                format!("failed to parse `{}` as filesystem path: {}", ca_dir, error)
            })?;

            let entries = fs::read_dir(&path)
                .map_err(|error| {
                    format!("failed to access directory `{}`: {}", path.display(), error)
                })?
                .into_iter()
                .map(|entry| entry.map_err(|error| error.to_string()))
                .collect::<Result<Vec<fs::DirEntry>, String>>()?;

            for entry in entries {
                let cert_path = entry.path();

                let mut reader = match fs::File::open(&cert_path) {
                    Ok(cert_file) => BufReader::new(cert_file),
                    Err(error) => {
                        return Err(format!(
                            "failed to open file `{}`: {}",
                            cert_path.display(),
                            error
                        ));
                    }
                };

                for cert in rustls_pemfile::certs(&mut reader) {
                    roots.add(cert.unwrap()).unwrap();
                }
            }
        }

        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::aws_lc_rs::default_provider(),
        );

        // Build a custom TLS configuration from the truststore that was created earlier.
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();

        // Initialize the agent used to communicate with pullconfd.
        let agent = AgentBuilder::new()
            .https_only(true)
            .tls_config(std::sync::Arc::new(tls_config))
            .build();

        // Both successful and erroneous responses from pullconfd are JSON. Except when
        // the response comes from an intermediary (e.g. a reverse proxy).
        let content_type = "application/json";

        let scope = "request";

        // Query pullconfd for this system's configuration and parse the result.
        let url = base_url
            .join(&format!("/api/clients/{}/resources", hostname))
            .unwrap();

        let mut request = agent
            .get(url.as_str())
            .set("accept", content_type)
            .set("x-api-key", &api_key);

        debug!(
            "(pid: {}) checking if a file with an etag of a saved resource list exists",
            pid
        );

        if let Some(etag) = get_etag(pid)? {
            debug!(
                "(pid: {}) adding etag of saved resource list to request",
                pid
            );
            request = request.set("if-none-match", &etag);
        }

        let _timer = Instant::now();

        debug!("(pid: {}) requesting resource list from `{}`", pid, url);

        let resources = match request.call().inspect(|response| {
            if let Some(content_length) = response.header("content-length") {
                debug!("(pid: {}) received {} bytes", pid, content_length);
            }

            debug!(
                "(pid: {}) finished request in {} ms",
                pid,
                (_timer.elapsed().as_millis() as f64) / 1000.0
            )
        }) {
            Ok(response) => {
                if response.status() == 304 {
                    debug!("(pid: {}) server returned 304, ignoring the request body and reading saved resource list from disk", pid);

                    get_saved_resource_list(pid)?.data
                } else {
                    // If the response is successful according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if response.content_type() != content_type {
                        return Err(format!(
                            "unexpected API response content type, expected `{}`, got `{}` and status `{} {}` from `{}`",

                            content_type,
                            response.content_type(),
                            response.status(),
                            response.status_text(),
                            response.header("server").unwrap_or_default()
                        ));
                    } else {
                        let etag = response.header("etag").map(|value| value.to_string());

                        debug!(
                            "(pid: {}) content type is `{}`, deserializing resource list",
                            pid, content_type
                        );

                        // Otherwise parse the payload as it is expected to be a JSON-encoded
                        // resource list.
                        let payload = response.into_string().map_err(|error| {
                            format!("failed to parse payload as utf-8 string: {}", error)
                        })?;

                        if let Some(etag) = etag {
                            debug!("(pid: {}) saving resource list to disk", pid);

                            save_resource_list(pid, &etag, &payload)?;
                        }

                        serde_json::from_str::<Resources>(&payload)
                            .map_err(|error| {
                                format!("failed to deserialize resource list : {}", error)
                            })?
                            .data
                    }
                }
            }
            Err(error) => match error {
                ureq::Error::Status(_, response) => {
                    // If the response is erroneous according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if response.content_type() != content_type {
                        return Err(format!(
                            "unexpected API response content type, expected `{}`, got `{}` and status `{} {}` from `{}`",
                            content_type,
                            response.content_type(),
                            response.status(),
                            response.status_text(),
                            response.header("server").unwrap_or_default()
                        ));
                    } else {
                        debug!(
                            "(pid: {}) content type is `{}`, deserializing error message",
                            pid, content_type
                        );

                        // Otherwise parse the well-known API error format from JSON and log
                        // the error appropiately. Then terminate the program.
                        let error = response.into_json::<Error>().map_err(|error| {
                            format!("failed to deserialize error response: {}", error)
                        })?;

                        return Err(format!(
                            "pullconfd failed to process the request: {}, {}",
                            error.title, error.detail
                        ));
                    }
                }
                // Log any unexpected errors as-is and terminate the program.
                ureq::Error::Transport(error) => {
                    return Err(format!(
                        "failed to send request to pullconfd: {}, {}",
                        error.message().unwrap(),
                        error.source().unwrap()
                    ));
                }
            },
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
    pub fn apply(mut self, pid: u32) {
        let _timer = Instant::now();

        let mut applied_resources = HashMap::with_capacity(self.resources.len());

        while let Some(mut resource) = self.resources.pop_front() {
            if !resource.is_ready(&applied_resources) {
                self.resources.push_back(resource);
                continue;
            }

            resource.apply(
                pid,
                &self.agent,
                &self.base_url,
                &self.api_key,
                &applied_resources,
            );

            applied_resources.insert(resource.id(), resource);
        }

        let _elapsed = (_timer.elapsed().as_millis() as f64) / 1000.0;

        info!("applied resource list in {:.3} seconds", _elapsed);
    }
}

fn get_etag(pid: u32) -> Result<Option<String>, String> {
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

fn get_saved_resource_list(pid: u32) -> Result<Resources, String> {
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

fn save_resource_list(pid: u32, etag: &str, data: &str) -> Result<(), String> {
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
