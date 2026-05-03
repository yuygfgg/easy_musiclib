#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="${MUSICLIB_ENV_FILE:-$repo_root/.env}"
managed_begin="# >>> easy_musiclib managed"
managed_end="# <<< easy_musiclib managed"

prompt() {
  local label="$1"
  local default="$2"
  local value
  if [[ -n "$default" ]]; then
    read -r -p "$label [$default]: " value
    printf '%s' "${value:-$default}"
  else
    read -r -p "$label: " value
    printf '%s' "$value"
  fi
}

prompt_yes_no() {
  local label="$1"
  local default="$2"
  local value suffix
  if [[ "$default" == "y" ]]; then
    suffix="[Y/n]"
  else
    suffix="[y/N]"
  fi
  while true; do
    read -r -p "$label $suffix: " value
    value="${value:-$default}"
    value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
    case "$value" in
      y|yes) return 0 ;;
      n|no) return 1 ;;
      *) printf 'Please answer y or n.\n' ;;
    esac
  done
}

prompt_tls_mode() {
  local default="$1"
  local value
  printf '\nTLS setup\n' >&2
  printf '  1) Generate a local self-signed certificate\n' >&2
  printf '  2) Use an existing certificate and private key\n' >&2
  if [[ "$default" == "keep" ]]; then
    printf '  3) Keep the current .env TLS settings\n' >&2
    printf '  4) Skip TLS for now\n' >&2
    while true; do
      read -r -p "Choose TLS setup [3]: " value
      value="${value:-3}"
      case "$value" in
        1|g|generate) printf 'generate'; return 0 ;;
        2|e|existing|use) printf 'existing'; return 0 ;;
        3|k|keep) printf 'keep'; return 0 ;;
        4|s|skip|none) printf 'skip'; return 0 ;;
        *) printf 'Please choose 1, 2, 3, or 4.\n' >&2 ;;
      esac
    done
  else
    printf '  3) Skip TLS for now\n' >&2
    while true; do
      read -r -p "Choose TLS setup [1]: " value
      value="${value:-1}"
      case "$value" in
        1|g|generate) printf 'generate'; return 0 ;;
        2|e|existing|use) printf 'existing'; return 0 ;;
        3|s|skip|none) printf 'skip'; return 0 ;;
        *) printf 'Please choose 1, 2, or 3.\n' >&2 ;;
      esac
    done
  fi
}

existing_value() {
  local key="$1"
  [[ -f "$env_file" ]] || return 0
  awk -v key="$key" '
    BEGIN { in_managed=0 }
    $0 == "# >>> easy_musiclib managed" { in_managed=1; next }
    $0 == "# <<< easy_musiclib managed" { in_managed=0; next }
    {
      line=$0
      sub(/^[[:space:]]*export[[:space:]]+/, "", line)
      if (line ~ "^[[:space:]]*" key "[[:space:]]*=") {
        sub("^[[:space:]]*" key "[[:space:]]*=[[:space:]]*", "", line)
        sub(/[[:space:]]+#.*$/, "", line)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
        first=substr(line, 1, 1)
        last=substr(line, length(line), 1)
        if ((first == "\047" && last == "\047") || (first == "\"" && last == "\"")) {
          line=substr(line, 2, length(line)-2)
        }
        print line
      }
    }
  ' "$env_file" | tail -n 1
}

default_value() {
  local key="$1"
  local fallback="$2"
  local existing env_value
  env_value="$(printenv "$key" 2>/dev/null || true)"
  existing="$(existing_value "$key")"
  printf '%s' "${env_value:-${existing:-$fallback}}"
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
  else
    printf '%s/%s' "$repo_root" "$path"
  fi
}

env_quote() {
  local value="$1"
  value="$(printf "%s" "$value" | sed "s/'/'\\\\''/g")"
  printf "'%s'" "$value"
}

remove_managed_block() {
  local source="$1"
  [[ -f "$source" ]] || return 0
  awk -v begin="$managed_begin" -v end="$managed_end" '
    $0 == begin { skip=1; next }
    $0 == end { skip=0; next }
    !skip { print }
  ' "$source"
}

openssl_available() {
  command -v openssl >/dev/null 2>&1
}

generate_tls_certificate() {
  local cert_path="$1"
  local key_path="$2"
  local subject_input="$3"
  local days="$4"

  openssl_available || {
    printf 'openssl is required to generate TLS certificates.\n' >&2
    return 1
  }

  mkdir -p "$(dirname "$cert_path")" "$(dirname "$key_path")"
  chmod 700 "$(dirname "$key_path")" 2>/dev/null || true

  local config_file
  config_file="$(mktemp)"

  local common_name san_entries dns_index ip_index item
  common_name="localhost"
  san_entries=""
  dns_index=1
  ip_index=1
  for item in $subject_input; do
    if [[ "$item" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ || "$item" == *:* ]]; then
      san_entries+="IP.$ip_index = $item"$'\n'
      ((ip_index++))
    else
      san_entries+="DNS.$dns_index = $item"$'\n'
      if [[ "$common_name" == "localhost" ]]; then
        common_name="$item"
      fi
      ((dns_index++))
    fi
  done

  cat >"$config_file" <<EOF
[ req ]
prompt = no
distinguished_name = dn
x509_extensions = v3_req

[ dn ]
CN = $common_name

[ v3_req ]
basicConstraints = critical, CA:FALSE
keyUsage = critical, digitalSignature
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[ alt_names ]
$san_entries
EOF

  openssl ecparam -name prime256v1 -genkey -noout -out "$key_path"
  chmod 600 "$key_path"
  openssl req -new -x509 -sha256 -days "$days" \
    -key "$key_path" \
    -out "$cert_path" \
    -config "$config_file"
  rm -f "$config_file"
}

write_env_file() {
  local tmp_file
  tmp_file="$(mktemp)"
  remove_managed_block "$env_file" >"$tmp_file"
  {
    printf '\n%s\n' "$managed_begin"
    printf 'MUSICLIB_DB=%s\n' "$(env_quote "$musiclib_db")"
    printf 'MUSICLIB_BIND=%s\n' "$(env_quote "$musiclib_bind")"
    printf 'MUSICLIB_STATIC_DIR=%s\n' "$(env_quote "$musiclib_static_dir")"
    if [[ -n "$tls_cert" && -n "$tls_key" ]]; then
      printf 'MUSICLIB_TLS_CERT=%s\n' "$(env_quote "$tls_cert")"
      printf 'MUSICLIB_TLS_KEY=%s\n' "$(env_quote "$tls_key")"
    fi
    printf 'RUST_LOG=%s\n' "$(env_quote "$rust_log")"
    printf '%s\n' "$managed_end"
  } >>"$tmp_file"
  mkdir -p "$(dirname "$env_file")"
  mv "$tmp_file" "$env_file"
  chmod 600 "$env_file"
}

printf 'Easy Musiclib environment setup\n'
printf 'Repository: %s\n\n' "$repo_root"

env_file="$(absolute_path "$(prompt ".env file" "$env_file")")"
musiclib_db="$(prompt "MUSICLIB_DB" "$(default_value MUSICLIB_DB "musiclib.db")")"
musiclib_bind="$(prompt "MUSICLIB_BIND" "$(default_value MUSICLIB_BIND "0.0.0.0:5010")")"
musiclib_static_dir="$(prompt "MUSICLIB_STATIC_DIR" "$(default_value MUSICLIB_STATIC_DIR "crates/web/dist")")"
rust_log="$(prompt "RUST_LOG" "$(default_value RUST_LOG "easy_musiclib_server=info,tower_http=info")")"

host_default="localhost 127.0.0.1"
if command -v hostname >/dev/null 2>&1; then
  host_name="$(hostname 2>/dev/null || true)"
  if [[ -n "$host_name" && "$host_name" != "localhost" ]]; then
    host_default="$host_default $host_name"
  fi
fi

current_tls_cert="$(default_value MUSICLIB_TLS_CERT "")"
current_tls_key="$(default_value MUSICLIB_TLS_KEY "")"
tls_cert=""
tls_key=""
tls_mode_default="generate"
if [[ -n "$current_tls_cert" && -n "$current_tls_key" ]]; then
  printf '\nCurrent TLS settings found:\n  cert: %s\n  key:  %s\n' "$current_tls_cert" "$current_tls_key"
  tls_mode_default="keep"
fi

tls_mode="$(prompt_tls_mode "$tls_mode_default")"
printf '\n'
case "$tls_mode" in
  keep)
    tls_cert="$(absolute_path "$current_tls_cert")"
    tls_key="$(absolute_path "$current_tls_key")"
    printf 'Keeping TLS settings:\n  cert: %s\n  key:  %s\n\n' "$tls_cert" "$tls_key"
    ;;
  generate)
    tls_cert="$(absolute_path "$(prompt "Certificate output path" "$(default_value MUSICLIB_TLS_CERT "$repo_root/certs/musiclib.crt")")")"
    tls_key="$(absolute_path "$(prompt "Private key output path" "$(default_value MUSICLIB_TLS_KEY "$repo_root/certs/musiclib.key")")")"
    tls_hosts="$(prompt "Certificate DNS/IP names separated by spaces" "$host_default")"
    tls_days="$(prompt "Certificate validity days" "397")"
    generate_tls_certificate "$tls_cert" "$tls_key" "$tls_hosts" "$tls_days"
    printf 'Generated TLS certificate:\n  cert: %s\n  key:  %s\n\n' "$tls_cert" "$tls_key"
    ;;
  existing)
    tls_cert="$(absolute_path "$(prompt "Existing certificate path" "$(default_value MUSICLIB_TLS_CERT "$repo_root/certs/musiclib.crt")")")"
    tls_key="$(absolute_path "$(prompt "Existing private key path" "$(default_value MUSICLIB_TLS_KEY "$repo_root/certs/musiclib.key")")")"
    printf 'Using existing TLS files:\n  cert: %s\n  key:  %s\n\n' "$tls_cert" "$tls_key"
    ;;
  skip)
    printf 'Skipping TLS. No MUSICLIB_TLS_* values will be written by the managed block.\n\n'
    ;;
esac

write_env_file

printf 'Wrote %s\n' "$env_file"
printf 'Start the server from the repository root; it will read .env automatically.\n'
printf 'After accounts exist, TLS variables in .env are required for startup.\n'
