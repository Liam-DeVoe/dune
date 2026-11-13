# Run `just` to list recipes.

default:
    @just --list

# Deploy the Rust server and the web client to https://treachery.exe.xyz/ (server built on the box).
deploy:
    deploy/deploy.sh

# Create the box from nothing (idempotent), then deploy.
provision:
    deploy/provision.sh

# Compile the Blazor client locally.
build:
    scripts/build-client.sh

# Publish the client to /tmp/treachery-client-<label> for screenshots (default label: after).
publish label="after":
    scripts/publish-client.sh {{label}}

# Regenerate a fixture from scripts/fixture-specs.json, or `just fixture describe <name>` to print what one holds.
fixture *args:
    DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH" dotnet run scripts/make-fixture.cs -- {{args}}

# Screenshot scenarios from the published client: `just shots --list`, `just shots ship`, `just shots --before`.
shots *args:
    uv run scripts/ui/shots.py {{args}}

# Serve the published client with the fixtures on http://127.0.0.1:8765 and print scene URLs to open by hand.
serve *args:
    uv run scripts/ui/serve.py {{args}}

# Write design/<area>/gallery.html pairing every before and after shot by name.
gallery area:
    uv run scripts/ui/gallery.py {{area}}

# Run the server locally on http://127.0.0.1:5050 with the client served from source.
run:
    cd treachery-online/Server && \
    DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH" \
    DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 \
    ASPNETCORE_URLS=http://127.0.0.1:5050 ASPNETCORE_ENVIRONMENT=Development \
    dotnet run --no-launch-profile

# Tail the service log on the box.
logs:
    ssh -i ~/.ssh/id_ed25519_exepersonal exedev@treachery.exe.xyz sudo journalctl -u treachery -f

# Format the SCSS under Client/Styles and the scoped .razor.scss files.
fmt-css:
    npx --yes prettier@3 --write "treachery-online/Client/**/*.scss"
