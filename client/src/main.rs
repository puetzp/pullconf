mod configuration;
mod resources;
mod util;

use std::io::Write;
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
    env_logger::builder()
        .format(move |buf, record| {
            writeln!(
                buf,
                "[{} {} {}] {}",
                buf.timestamp_millis(),
                pid,
                record.level(),
                record.args()
            )
        })
        .init();

    log::info!("starting {} v{}", APPLICATION, VERSION);

    if !nix::unistd::getuid().is_root() {
        log::error!("pullconf must be executed as root");
        return ExitCode::FAILURE;
    }

    // Fetch the client configuration from pullconfd and apply it.
    match configuration::Configuration::get() {
        Ok(configuration) => {
            let report = configuration.apply();

            let exit = report
                .resources
                .iter()
                .any(|resource| resource.is_failed())
                .then_some(ExitCode::FAILURE)
                .unwrap_or_default();

            match serde_json::to_string(&report) {
                Ok(json) => {
                    println!("{}", json);
                    exit
                }
                Err(error) => {
                    log::error!("failed to serialize report to JSON: {}", error);
                    ExitCode::FAILURE
                }
            }
        }
        Err(error) => {
            log::error!("{}", error);
            ExitCode::FAILURE
        }
    }
}
