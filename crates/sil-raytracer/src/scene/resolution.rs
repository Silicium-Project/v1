//! The image size a scene declares: `resolution = { width = W height = H }`, read the way the
//! site's editor reads it (case-insensitive, width before height inside the same block).

/// Width and height of the scene's `resolution` block, or the engine default (1920 × 1080) when the
/// text declares none. Both are at least 1.
pub fn scene_resolution(text: &str) -> (usize, usize) {
    declared(text).unwrap_or((1920, 1080))
}

fn declared(text: &str) -> Option<(usize, usize)> {
    // ASCII lowercasing keeps every byte offset, so positions found here index `text` too.
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find("resolution") {
        let start = from + found + "resolution".len();
        from = start;
        let rest = lower[start..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('{') else {
            continue;
        };
        let block = rest.split('}').next().unwrap_or("");
        let width = number_after(block, "width")?;
        let after_width = block.split_once("width").map(|(_, tail)| tail)?;
        let height = number_after(after_width, "height")?;
        return Some((width.max(1), height.max(1)));
    }
    None
}

/// The number after `key` and `=`, when `key` is followed by `=` and digits.
fn number_after(text: &str, key: &str) -> Option<usize> {
    let tail = text.split_once(key).map(|(_, tail)| tail)?;
    let tail = tail.trim_start().strip_prefix('=')?.trim_start();
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}
