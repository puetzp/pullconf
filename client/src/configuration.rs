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
const CATALOG_FILE: &str = "/var/lib/pullconf/catalog";

/// This struct contains every piece of information that is needed to retrieve
/// this system's configuration (resource catalog) from pullconfd and apply it.
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
    /// catalog are parsed from JSON and then returned.
    pub fn get(pid: u32) -> Result<Self, Terminate> {
        let scope = "configuration";

        // Retrieve the system's (fully-qualified) hostname. The hostname is used
        // to query pullconfd for this system's configuration.
        let hostname = {
            let mut command = Command::new("hostname");
            command.arg("--fqdn");

            let result = match command.output() {
                Ok(r) => r,
                Err(error) => {
                    error!(scope, pid; "failed to execute {:?}: {}", command, error);
                    return Err(Terminate);
                }
            };

            if result.status.success() {
                let output = match String::from_utf8(result.stdout) {
                    Ok(stdout) => stdout,
                    Err(error) => {
                        error!(scope, pid; "failed to read stdout from {:?}: {}", command, error);
                        return Err(Terminate);
                    }
                };

                match Hostname::from_str(output.as_str().trim()) {
                    Ok(hostname) => hostname,
                    Err(error) => {
                        error!(scope, pid; "failed to parse output from {:?}: {}", command, error);
                        return Err(Terminate);
                    }
                }
            } else {
                error!(
                    scope,
                    pid;
                    "failed to execute {:?}, returned non-zero exit code",
                    command
                );
                return Err(Terminate);
            }
        };

        let base_url = {
            let address = {
                let v = "PULLCONF_SERVER";
                match env::var(v) {
                    Ok(value) => format!("https://{}", value),
                    Err(error) => {
                        error!(scope, pid; "failed to read environment variable {}: {}", v, error);
                        return Err(Terminate);
                    }
                }
            };

            match Url::parse(&address) {
                Ok(url) => url,
                Err(error) => {
                    error!(scope, pid; "failed to parse {} as URL: {}", address, error);
                    return Err(Terminate);
                }
            }
        };

        // The API key that is defined in the TOML configuration file on the server.
        let api_key = {
            let v = "PULLCONF_API_KEY";
            match env::var(v) {
                Ok(value) => value,
                Err(error) => {
                    error!(scope, pid; "failed to read environment variable {}: {}", v, error);
                    return Err(Terminate);
                }
            }
        };

        // Add common CA certificates to the truststore of this request.
        let mut roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        // If a custom directory path is provided that contains other (e.g. self-signed)
        // CA certificates, parse every certificate in each file and add them to
        // the truststore as well.
        if let Ok(ca_dir) = env::var("PULLCONF_CA_DIR") {
            let path = match PathBuf::from_str(&ca_dir) {
                Ok(path) => path,
                Err(error) => {
                    error!(scope, pid; "failed to parse {} as filesystem path: {}", ca_dir, error);
                    return Err(Terminate);
                }
            };

            let entries = match fs::read_dir(&path) {
                Ok(e) => e,
                Err(error) => {
                    error!(scope, pid; "failed to access directory {}: {}", path.display(), error);
                    return Err(Terminate);
                }
            };

            for entry in entries {
                let cert_path = match entry {
                    Ok(entry) => entry.path(),
                    Err(error) => {
                        error!(pid, scope; "{}", error);
                        return Err(Terminate);
                    }
                };

                let mut reader = match fs::File::open(&cert_path) {
                    Ok(cert_file) => BufReader::new(cert_file),
                    Err(error) => {
                        error!(pid, scope; "failed to open file {}: {}", cert_path.display(), error);
                        return Err(Terminate);
                    }
                };

                for cert in rustls_pemfile::certs(&mut reader) {
                    roots.add(cert.unwrap()).unwrap();
                }
            }
        }

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

        debug!(scope, pid, url:%; "checking if a file with an etag of a saved resource catalog exists");

        if let Some(etag) = get_etag(pid)? {
            debug!(scope, pid, url:%; "adding etag of saved resource catalog to request");
            request = request.set("if-none-match", &etag);
        }

        let _timer = Instant::now();

        let resources = match request.call().inspect(|response| {
            if let Some(content_length) = response.header("content-length") {
                debug!(scope, pid, url:%; "received {} bytes", content_length);
            }

            debug!(scope, pid, url:%;
                   "finished request in {} ms",
                   (_timer.elapsed().as_millis() as f64) / 1000.0
            )
        }) {
            Ok(response) => {
                if response.status() == 304 {
                    debug!(scope, pid, url:%; "server returned 304, ignoring the request body and reading saved resource catalog from disk");

                    get_saved_resource_catalog(pid)?.data
                } else {
                    // If the response is successful according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if response.content_type() != content_type {
                        error!(
                            scope,
                            pid,
                            url:%;
                            "unexpected API response content type, expected {}, got {} and status {} {} from {}",
                            content_type,
                            response.content_type(),
                            response.status(),
                            response.status_text(),
                            response.header("server").unwrap_or_default()
                        );
                        return Err(Terminate);
                    } else {
                        let etag = response.header("etag").map(|value| value.to_string());

                        debug!(scope, pid, url:%; "content type is {}, deserializing resource catalog", content_type);

                        // Otherwise parse the payload as it is expected to be a JSON-encoded
                        // resource catalog.
                        let payload = match response.into_string() {
                            Ok(s) => s,
                            Err(error) => {
                                error!(scope, pid, url:%; "failed to parse payload as utf-8 string: {}", error);
                                return Err(Terminate);
                            }
                        };

                        if let Some(etag) = etag {
                            debug!(scope, pid, url:%; "saving resource catalog data to disk");

                            save_resource_catalog(pid, &etag, &payload)?;
                        }

                        match serde_json::from_str::<Resources>(&payload) {
                            Ok(catalog) => catalog.data,
                            Err(error) => {
                                error!(scope, pid, url:%; "failed to deserialize resource catalog : {}", error);
                                return Err(Terminate);
                            }
                        }
                    }
                }
            }
            Err(error) => match error {
                ureq::Error::Status(_, response) => {
                    // If the response is erroneous according to the status code, but the
                    // content type hints at a non-JSON body, log a generic error including
                    // relevant information for debugging and terminate the program.
                    if response.content_type() != content_type {
                        error!(
                            scope,
                            pid,
                            url:%;
                            "unexpected API response content type, expected {}, got {} and status {} {} from {}",
                            content_type,
                            response.content_type(),
                            response.status(),
                            response.status_text(),
                            response.header("server").unwrap_or_default()
                        );
                        return Err(Terminate);
                    } else {
                        debug!(scope, pid, url:%; "content type is {}, deserializing error message", content_type);

                        // Otherwise parse the well-known API error format from JSON and log
                        // the error appropiately. Then terminate the program.
                        let error = match response.into_json::<Error>() {
                            Ok(error) => error,
                            Err(error) => {
                                error!(scope, pid, url:%; "failed to deserialize error response: {}", error);
                                return Err(Terminate);
                            }
                        };

                        error!(
                            scope,
                            pid,
                            url:%
                            ;
                            "pullconfd failed to process the request: ({}) {}",
                            error.title,
                            error.detail
                        );

                        return Err(Terminate);
                    }
                }
                // Log any unexpected errors as-is and terminate the program.
                ureq::Error::Transport(error) => {
                    error!(scope, pid, url:%; "{}", error.source().unwrap());
                    return Err(Terminate);
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

        info!(pid; "applied resource catalog in {:.3} seconds", _elapsed);
    }
}

fn get_etag(pid: u32) -> Result<Option<String>, Terminate> {
    match fs::read_to_string(ETAG_FILE) {
        Ok(etag) => {
            if etag.is_empty() {
                Ok(None)
            } else {
                Ok(Some(etag))
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            debug!(scope = "request", pid; "etag file does not exist");
            Ok(None)
        }
        Err(error) => {
            error!(scope = "request", pid; "failed to read etag file {}: {}", ETAG_FILE, error);
            Err(Terminate)
        }
    }
}

fn get_saved_resource_catalog(pid: u32) -> Result<Resources, Terminate> {
    match fs::read_to_string(CATALOG_FILE) {
        Ok(s) => match serde_json::from_str::<Resources>(&s) {
            Ok(resources) => Ok(resources),
            Err(error) => {
                error!(scope = "request", pid; "failed to deserialize resource catalog from file: {}", error);
                Err(Terminate)
            }
        },
        Err(error) => {
            error!(scope = "request", pid; "failed to read resource catalog file: {}", error);
            Err(Terminate)
        }
    }
}

fn save_resource_catalog(pid: u32, etag: &str, catalog: &str) -> Result<(), Terminate> {
    if let Err(error) = fs::write(ETAG_FILE, etag) {
        error!(
            scope = "request",
            pid;
            "failed to save latest resource catalog etag to file: {}",
            error
        );

        return Err(Terminate);
    }

    if let Err(error) = fs::write(CATALOG_FILE, catalog) {
        error!(
            scope = "request",
            pid;
            "failed to save latest resource catalog to file: {}",
            error
        );

        return Err(Terminate);
    }

    Ok(())
}
