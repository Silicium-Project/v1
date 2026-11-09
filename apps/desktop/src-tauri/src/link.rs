//! Reads the link the website opens: `silicium://link?code=…`. Only the code is taken from it;
//! any other parameter, a second code or a malformed code is refused. The code format matches
//! the API (services/api/internal/devices): 12 characters, no 0, O, 1 or I.

/// Starts of every accepted link. The scheme is registered by the installer (tauri.conf.json,
/// plugins.deep-link). `url::Url` normalises an empty path to `/`, so both forms reach us.
const PREFIXES: [&str; 2] = ["silicium://link?", "silicium://link/?"];
const CODE_LENGTH: usize = 12;
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The link code in `raw`, in upper case, when `raw` is a well-formed link.
pub fn code_from_url(raw: &str) -> Option<String> {
    let query = PREFIXES
        .iter()
        .find_map(|prefix| raw.strip_prefix(prefix))?
        .trim_end_matches('/');
    let mut codes = Vec::new();
    for pair in query.split('&') {
        codes.push(pair.strip_prefix("code=")?);
    }
    let [code] = codes.as_slice() else {
        return None;
    };
    let code = code.to_ascii_uppercase();
    is_code(&code).then_some(code)
}

/// True for a link code in upper case, as the API issues it.
pub fn is_code(code: &str) -> bool {
    code.len() == CODE_LENGTH && code.bytes().all(|byte| ALPHABET.contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_yields_its_code_in_upper_case() {
        assert_eq!(
            code_from_url("silicium://link?code=abcdefghjklm"),
            Some("ABCDEFGHJKLM".to_string())
        );
        assert_eq!(
            code_from_url("silicium://link?code=ABCDEFGHJKLM/"),
            Some("ABCDEFGHJKLM".to_string())
        );
    }

    #[test]
    fn the_form_url_crate_produces_is_accepted() {
        // Windows hands the URL over normalised by url::Url: the empty path becomes "/".
        assert_eq!(
            code_from_url("silicium://link/?code=BCDFGHJKMNPQ"),
            Some("BCDFGHJKMNPQ".to_string())
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for raw in [
            "https://link?code=ABCDEFGHJKLM",
            "silicium://other?code=ABCDEFGHJKLM",
            "silicium://link?",
            "silicium://link?code=ABCDEFGHJKL",
            "silicium://link?code=ABCDEFGHJKL0",
            "silicium://link?code=ABCDEFGHJKLI",
            "silicium://link?code=ABCDEFGHJKLM&label=pc",
            "silicium://link?code=ABCDEFGHJKLM&code=ABCDEFGHJKLN",
        ] {
            assert_eq!(code_from_url(raw), None, "{raw}");
        }
    }

    #[test]
    fn only_the_alphabet_is_a_code() {
        assert!(is_code("BCDFGHJKMNPQ"));
        assert!(!is_code("BCDFGHJKMNP"));
        assert!(!is_code("BCDFGHJKMNPO"));
    }
}
