mod configuration;
mod env;
mod handlers;
mod types;

use crate::configuration::Configuration;
use log::{debug, error, info, warn};
use rouille::Server;
use signal_hook::{consts::signal::*, iterator::Signals};
use std::{
    fs,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APPLICATION: &str = env!("CARGO_PKG_NAME");

// Type alias for the state data structure that is shared among threads.
type SharedAppState = Arc<RwLock<AppState>>;

// Data structure containing shared data that is relevant to all handlers and
// tasks.
pub struct AppState {
    configuration: Configuration,
    resources: PathBuf,
    assets: PathBuf,
}

impl AppState {
    pub fn initialize() -> Result<Self, String> {
        let assets = env::parse_path(
            env::FileType::Directory,
            "PULLCONF_ASSET_DIR",
            "/etc/pullconfd/assets",
        )?;

        let resources = env::parse_path(
            env::FileType::Directory,
            "PULLCONF_RESOURCE_DIR",
            "/etc/pullconfd/conf.d",
        )?;

        // Todo: Every function should return `String` to propagate errors.
        let configuration = Configuration::try_from(&resources)?;

        let state = AppState {
            configuration,
            resources,
            assets,
        };

        Ok(state)
    }
}

fn main() -> ExitCode {
    // Initialize logging.
    env_logger::init();

    info!("starting {} v{}", APPLICATION, VERSION);

    // Initialize the shared data structure.
    let state = {
        match AppState::initialize() {
            Ok(state) => Arc::new(RwLock::new(state)),
            Err(error) => {
                error!("{}", error);
                return ExitCode::FAILURE;
            }
        }
    };

    // Create a server and bind to a socket that listens for incoming connections.
    let server = {
        let _state = state.clone();

        let socket = match env::parse_socket("PULLCONF_LISTEN_ON", "127.0.0.1:443") {
            Ok(path) => path,
            Err(error) => {
                error!("{}", error);
                return ExitCode::FAILURE;
            }
        };

        let certificate = {
            let path = match env::parse_path(
                env::FileType::File,
                "PULLCONF_TLS_CERTIFICATE",
                "/etc/pullconfd/tls/server.crt",
            ) {
                Ok(path) => path,
                Err(error) => {
                    error!("{}", error);
                    return ExitCode::FAILURE;
                }
            };

            match fs::read_to_string(&path) {
                Ok(content) => content.as_bytes().to_vec(),
                Err(error) => {
                    error!(
                        "`{}`: failed to read TLS certificate from file: {}",
                        path.display(),
                        error
                    );
                    return ExitCode::FAILURE;
                }
            }
        };

        let key = {
            let path = match env::parse_path(
                env::FileType::File,
                "PULLCONF_TLS_PRIVATE_KEY",
                "/etc/pullconfd/tls/server.key",
            ) {
                Ok(path) => path,
                Err(error) => {
                    error!("{}", error);
                    return ExitCode::FAILURE;
                }
            };

            match fs::read_to_string(&path) {
                Ok(content) => content.as_bytes().to_vec(),
                Err(error) => {
                    error!(
                        "`{}`: failed to read TLS private key from file: {}",
                        path.display(),
                        error
                    );
                    return ExitCode::FAILURE;
                }
            }
        };

        match Server::new_ssl(
            socket,
            move |request| handlers::handle_request(request, _state.clone()),
            certificate,
            key,
        ) {
            Ok(server) => {
                info!("server is accepting connections at `{}`", socket);
                server
            }
            Err(error) => {
                error!("failed to start server: {}", error);
                return ExitCode::FAILURE;
            }
        }
    };

    // Dispatch a new thread with the server that can be gracefully stopped.
    let (handle, sender) = server.stoppable();

    // Create another thread which listens for incoming termination signals
    // and sends a message to the web server thread to initiate graceful shutdown.
    // Also listen for SIGHUP which prompts a configuration reload.
    let _state = state.clone();

    thread::spawn(move || {
        let mut signals = Signals::new([SIGTERM, SIGINT, SIGHUP]).unwrap();

        'outer: loop {
            if let Some(signal) = signals.pending().next() {
                debug!("received signal `{}`", signal);

                match signal {
                    SIGTERM | SIGINT => {
                        if let Err(error) = sender.send(()) {
                            error!(
                                "failed to forward shutdown signal `{}` for graceful shutdown: {}",
                                signal, error
                            );
                        }

                        break 'outer;
                    }
                    SIGHUP => {
                        let mut state = match _state.write() {
                            Ok(s) => s,
                            Err(error) => {
                                error!("failed to acquire write access to reload shared application state: {}", error);
                                continue;
                            }
                        };

                        match Configuration::try_from(&state.resources) {
                            Ok(configuration) => {
                                info!("successfully reloaded configuration",);
                                state.configuration = configuration;
                            }
                            Err(error) => {
                                error!("{}", error);
                                warn!("keeping the current configuration as reload failed",);
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    if let Err(error) = handle.join() {
        error!(
            "failed to join signal handler thread after shutdown signal was received: {:?}",
            error
        );
    }

    info!("shutting down gracefully");

    ExitCode::SUCCESS
}
