# Run `just` to list recipes.

default:
    @just --list

# Deploy the Rust server and the web client to https://treachery.exe.xyz/ (server built on the box).
deploy:
    deploy/deploy.sh

# Create the box from nothing (idempotent), then deploy.
provision:
    deploy/provision.sh

# Run the Rust server locally on http://127.0.0.1:5000 with dev scenes on; `cd web && npm run dev` serves the client against it.
run:
    cd rust && TREACHERY_DEV_SCENE=true cargo run --release -p treachery-server

# Tail the service log on the box.
logs:
    ssh -i ~/.ssh/id_ed25519_exepersonal exedev@treachery.exe.xyz sudo journalctl -u treachery -f

# Format the SCSS under web/styles.
fmt-css:
    npx --yes prettier@3 --write "web/styles/**/*.scss"
