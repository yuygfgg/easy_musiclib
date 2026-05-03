use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RuntimeEnv {
    values: BTreeMap<String, String>,
}

impl RuntimeEnv {
    pub fn load_default() -> Result<Self> {
        let path = std::env::var_os("MUSICLIB_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".env"));
        let values = match std::fs::read_to_string(&path) {
            Ok(raw) => parse_dotenv(&raw)
                .with_context(|| format!("failed to parse env file {}", path.display()))?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(err) => {
                return Err(err).with_context(|| format!("failed to read {}", path.display()));
            }
        };
        Ok(Self { values })
    }

    pub fn get(&self, key: &str) -> Option<String> {
        std::env::var(key)
            .ok()
            .or_else(|| self.values.get(key).cloned())
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }

    pub fn path_or(&self, key: &str, default: &str) -> PathBuf {
        self.get(key)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default))
    }
}

fn parse_dotenv(raw: &str) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (index, line) in raw.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((key, value)) = assignment.split_once('=') else {
            bail!("line {line_no}: expected KEY=VALUE");
        };
        let key = key.trim();
        validate_key(key).with_context(|| format!("line {line_no}: invalid key"))?;
        values.insert(key.to_string(), parse_value(value.trim(), line_no)?);
    }
    Ok(values)
}

fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        bail!("key is empty");
    }
    let mut chars = key.chars();
    let first = chars.next().unwrap_or_default();
    if !(first == '_' || first.is_ascii_alphabetic()) {
        bail!("key must start with a letter or underscore");
    }
    if chars.any(|ch| !(ch == '_' || ch.is_ascii_alphanumeric())) {
        bail!("key must contain only letters, numbers, and underscores");
    }
    Ok(())
}

fn parse_value(value: &str, line_no: usize) -> Result<String> {
    if let Some(rest) = value.strip_prefix('"') {
        parse_quoted(rest, line_no, '"', true)
    } else if let Some(rest) = value.strip_prefix('\'') {
        parse_quoted(rest, line_no, '\'', false)
    } else {
        Ok(strip_unquoted_comment(value).trim().to_string())
    }
}

fn parse_quoted(value: &str, line_no: usize, quote: char, escapes: bool) -> Result<String> {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == quote {
            let rest = chars.as_str().trim();
            if !rest.is_empty() && !rest.starts_with('#') {
                bail!("line {line_no}: unexpected characters after quoted value");
            }
            return Ok(out);
        }
        if escapes && ch == '\\' {
            let Some(next) = chars.next() else {
                bail!("line {line_no}: trailing escape in quoted value");
            };
            out.push(match next {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                other => other,
            });
        } else {
            out.push(ch);
        }
    }
    bail!("line {line_no}: unterminated quoted value")
}

fn strip_unquoted_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] == b'#' && (index == 0 || bytes[index - 1].is_ascii_whitespace()) {
            return &value[..index];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv_assignments() {
        let values = parse_dotenv(
            r#"
            # comment
            MUSICLIB_DB=musiclib.db
            export MUSICLIB_BIND="127.0.0.1:5010"
            MUSICLIB_STATIC_DIR='crates/web/dist'
            MUSICLIB_TLS_CERT=/tmp/cert.pem # comment
            "#,
        )
        .unwrap();
        assert_eq!(values.get("MUSICLIB_DB").unwrap(), "musiclib.db");
        assert_eq!(values.get("MUSICLIB_BIND").unwrap(), "127.0.0.1:5010");
        assert_eq!(
            values.get("MUSICLIB_STATIC_DIR").unwrap(),
            "crates/web/dist"
        );
        assert_eq!(values.get("MUSICLIB_TLS_CERT").unwrap(), "/tmp/cert.pem");
    }

    #[test]
    fn rejects_invalid_lines() {
        assert!(parse_dotenv("MUSICLIB_DB").is_err());
        assert!(parse_dotenv("1BAD=value").is_err());
        assert!(parse_dotenv("BAD=\"unterminated").is_err());
    }
}
