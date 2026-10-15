use std::collections::HashMap;

const HINT_CHARS: &[char] = &[
    'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l',
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',
];

/// Generates mnemonic two-letter hint labels for all visible interactive Ref IDs
pub fn generate_hints(ref_ids: &[u32]) -> HashMap<u32, [char; 2]> {
    let mut map = HashMap::new();
    let alphabet_len = HINT_CHARS.len();

    for (idx, &ref_id) in ref_ids.iter().enumerate() {
        let first = HINT_CHARS[(idx / alphabet_len) % alphabet_len];
        let second = HINT_CHARS[idx % alphabet_len];
        map.insert(ref_id, [first, second]);
    }
    map
}
