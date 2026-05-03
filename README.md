# Easy Musiclib

## Quick Start

```bash
scripts/setup-env.sh
cargo run --release -p easy_musiclib_server
```

`scripts/setup-env.sh` creates or updates `.env` in the repository root. The
server reads that file automatically on startup, so normal local runs do not need
manual `MUSICLIB_*` environment variables.

For later runs, rerun only `cargo run --release -p easy_musiclib_server` unless
you want to change the generated configuration.

The setup script can also generate a local ECDSA self-signed certificate and
write `MUSICLIB_TLS_CERT` / `MUSICLIB_TLS_KEY` for you. With the default TLS
setup, open:

`https://127.0.0.1:5010/`

For the generated self-signed certificate, use the browser's manual certificate
exception to continue. If you skipped TLS setup, open `http://127.0.0.1:5010/`
instead.

## Build

```bash
cargo build -p easy_musiclib_server
cargo test
cargo check -p easy_musiclib_web --target wasm32-unknown-unknown
cargo build -p easy_musiclib_web --target wasm32-unknown-unknown --release
wasm-bindgen \
  --target web \
  --out-dir crates/web/dist \
  --out-name easy_musiclib_web \
  target/wasm32-unknown-unknown/release/easy_musiclib_web.wasm
```

## Advanced Configuration

The server reads `.env` from the repository root on startup. Real environment
variables override values from `.env`, and `MUSICLIB_ENV_FILE=/path/to/.env`
loads a different env file.

`MUSICLIB_ENV_FILE` is checked before any env file is read, so it must be set in
the process environment. Putting `MUSICLIB_ENV_FILE=...` inside `.env` does not
make the server load a second env file.

Use explicit environment variables when you want to override the generated
configuration for one run:

```bash
MUSICLIB_DB=musiclib.db \
MUSICLIB_BIND=0.0.0.0:5010 \
MUSICLIB_STATIC_DIR=crates/web/dist \
MUSICLIB_TLS_CERT=/path/to/fullchain.pem \
MUSICLIB_TLS_KEY=/path/to/privkey.pem \
cargo run --release -p easy_musiclib_server
```

Set `MUSICLIB_HSTS=1` only when serving with a certificate already trusted by
your clients. HSTS is disabled by default so browsers can use a manual
certificate exception for local self-signed certificates.

## Accounts and TLS

The server starts open by default. When no accounts exist, anyone who can reach
the server can browse the library. Add accounts from the settings page to require
login for API, artwork, stream, HLS, and download requests.

Account credentials and authenticated media traffic require HTTPS. If accounts
exist, the server refuses to start without a certificate and private key. Use the
Quick Start setup script for local TLS, or provide `MUSICLIB_TLS_CERT` /
`MUSICLIB_TLS_KEY` through `.env` or the advanced environment-variable override.

Passwords are stored with Argon2id hashes, and session cookies are `HttpOnly`,
`Secure`, and `SameSite=Strict`.

## Scan

Use the settings page, or call the API:

```bash
curl -X POST http://127.0.0.1:5010/api/scan-jobs \
  -H 'content-type: application/json' \
  --data '{"roots":["/Volumes/smb/media/music"]}'
```

Poll:

```bash
curl http://127.0.0.1:5010/api/scan-jobs/1
```

## Import Old JSON

To migrate the old Python save file, use a new empty SQLite database path:

```bash
MUSICLIB_DB=musiclib.db \
cargo run --release -p easy_musiclib_server -- import-json library_data.json
```

The importer prints progress to stderr. By default it only migrates JSON data
and stored file paths; it does not call `stat` on every audio file. If you
explicitly want to record file size and mtime during import, add `--stat-files`:

```bash
MUSICLIB_DB=musiclib.db \
cargo run --release -p easy_musiclib_server -- import-json library_data.json --stat-files
```
