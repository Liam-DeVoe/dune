//! Runs the server with the settings in the environment (`settings.rs`).

fn main() {
    let result = treachery_server::settings::Settings::from_env().and_then(treachery_server::http::run);
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
