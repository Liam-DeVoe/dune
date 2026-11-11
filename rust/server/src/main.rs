//! Port of `Server/Program.cs` and `Server/Startup.cs`.

use treachery_server::{config::Config, runtime};

fn main() {
    // C#: Message.DefaultDescriber = DefaultSkin.Default; (Program.cs:20). See render.rs.
    let config = Config::load_from_current_dir();
    if let Err(e) = runtime::run(config) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
