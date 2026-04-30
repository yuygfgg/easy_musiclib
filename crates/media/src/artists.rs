use crate::normalize::normalize_name;

pub const DEFAULT_SPLIT_EXCEPTIONS: &[&str] =
    &["cool&create", "Factory Noise&AG", "Sing, R. Sing!"];

const DELIMITERS: &[&str] = &[
    "/", "／", "&", "＆", " x ", ";", "；", ",", "，", "×", "　", "、",
];

pub fn parse_artists(raw: &[String], exceptions: &[String]) -> Vec<String> {
    let mut merged_exceptions: Vec<String> = DEFAULT_SPLIT_EXCEPTIONS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    merged_exceptions.extend(exceptions.iter().cloned());
    let exception_norms: Vec<String> = merged_exceptions
        .iter()
        .map(|s| normalize_name(s, false))
        .collect();

    let mut parsed = Vec::new();
    for artist in raw {
        let mut remaining = artist.trim().to_string();
        while !remaining.is_empty() {
            let remaining_norm = normalize_name(&remaining, false);
            let mut matched = None;
            for (idx, ignored_norm) in exception_norms.iter().enumerate() {
                if let Some(pos) = remaining_norm.find(ignored_norm) {
                    matched = Some((idx, pos, ignored_norm.chars().count()));
                    break;
                }
            }
            if let Some((idx, pos_chars, len_chars)) = matched {
                let byte_pos = char_to_byte_idx(&remaining, pos_chars);
                let byte_end = char_to_byte_idx(&remaining, pos_chars + len_chars);
                let before = remaining[..byte_pos].trim();
                parsed.extend(split_and_clean(before));
                parsed.push(merged_exceptions[idx].clone());
                remaining = remaining[byte_end..].trim().to_string();
            } else {
                parsed.extend(split_and_clean(&remaining));
                break;
            }
        }
    }
    dedupe_keep_order(parsed)
}

fn split_and_clean(text: &str) -> Vec<String> {
    let mut parts = vec![text.to_string()];
    for delimiter in DELIMITERS {
        let mut next = Vec::new();
        for part in parts {
            next.extend(
                part.split(delimiter)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
        parts = next;
    }
    parts
}

fn dedupe_keep_order(items: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in items {
        let norm = normalize_name(&item, false);
        if seen.insert(norm) {
            out.push(item);
        }
    }
    out
}

fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_exceptions() {
        let got = parse_artists(
            &[
                "cool&create / A, B".to_string(),
                "Sing, R. Sing!".to_string(),
            ],
            &[],
        );
        assert_eq!(got, vec!["cool&create", "A", "B", "Sing, R. Sing!"]);
    }
}
