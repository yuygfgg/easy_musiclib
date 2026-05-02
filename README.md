# Easy Musiclib

## Build

```bash
cargo build -p easy_musiclib_server
cargo test
```

Leptos source can be checked separately:

```bash
cargo check -p easy_musiclib_web --target wasm32-unknown-unknown
```

Build the browser bundle with `wasm-bindgen`:

```bash
cargo build -p easy_musiclib_web --target wasm32-unknown-unknown --release
wasm-bindgen \
  --target web \
  --out-dir crates/web/dist \
  --out-name easy_musiclib_web \
  target/wasm32-unknown-unknown/release/easy_musiclib_web.wasm
```

## Run

```bash
cargo build -p easy_musiclib_server --release

MUSICLIB_DB=musiclib.db \
MUSICLIB_BIND=0.0.0.0:5010 \
MUSICLIB_STATIC_DIR=crates/web/dist \
target/release/easy_musiclib_server
```

Open `http://127.0.0.1:5010/`.

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
target/release/easy_musiclib_server import-json library_data.json
```

The importer prints progress to stderr. By default it only migrates JSON data and stored file paths; it does not call `stat` on every audio file. If you explicitly want to record file size and mtime during import, add `--stat-files`:

```bash
MUSICLIB_DB=musiclib.db \
target/release/easy_musiclib_server import-json library_data.json --stat-files
```
