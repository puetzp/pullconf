mod configuration;
mod resources;
mod util;

use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APPLICATION: &str = env!("CARGO_PKG_NAME");

fn main() -> ExitCode {
    // Create a new lifecycle ID that will be attached to every emitted
    // log output. Since this program is designed to run repeatedly via
    // some external scheduling mechanism (e.g. systemd timers), this ID
    // identifies all logs that were emitted during one iteration of the
    // program.
    // The ID will also be passed to every module and function that emits
    // logs.
    let pid = std::process::id();

    // Initialize logging.
    env_logger::init();

    log::info!("starting {} v{}", APPLICATION, VERSION);

    if !nix::unistd::getuid().is_root() {
        log::error!("(pid: {}) pullconf must be executed as root", pid);
        return ExitCode::FAILURE;
    }

    // Fetch the client configuration from pullconfd and apply it.
    match configuration::Configuration::get(pid) {
        Ok(configuration) => {
            configuration.apply(pid);
            ExitCode::SUCCESS
        }
        Err(error) => {
            log::error!("(pid: {}) {}", pid, error);
            ExitCode::FAILURE
        }
    }
}
