use std::collections::HashMap;

pub const HINT_CHARS: &[char] = &[
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

/// Resolves a two-character Vimium hint sequence back to its interactive Ref ID.
pub fn resolve_hint(hints: &HashMap<u32, [char; 2]>, target: [char; 2]) -> Option<u32> {
    for (&ref_id, &chars) in hints {
        if chars == target {
            return Some(ref_id);
        }
    }
    None
}

/// Filters hints that start with a given character prefix (for progressive two-key selection).
pub fn filter_hints(hints: &HashMap<u32, [char; 2]>, first_char: char) -> Vec<(u32, [char; 2])> {
    hints
        .iter()
        .filter(|(_, &chars)| chars[0] == first_char)
        .map(|(&k, &v)| (k, v))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hints() {
        let empty: Vec<u32> = Vec::new();
        let hints = generate_hints(&empty);
        assert!(hints.is_empty());
        assert_eq!(resolve_hint(&hints, ['a', 'a']), None);
        assert!(filter_hints(&hints, 'a').is_empty());
    }

    #[test]
    fn test_hint_generation_and_resolution() {
        let ref_ids = vec![101, 102, 103, 104];
        let hints = generate_hints(&ref_ids);

        assert_eq!(hints.len(), 4);
        assert_eq!(hints.get(&101), Some(&['a', 'a']));
        assert_eq!(hints.get(&102), Some(&['a', 's']));
        assert_eq!(hints.get(&103), Some(&['a', 'd']));
        assert_eq!(hints.get(&104), Some(&['a', 'f']));

        assert_eq!(resolve_hint(&hints, ['a', 'a']), Some(101));
        assert_eq!(resolve_hint(&hints, ['a', 's']), Some(102));
        assert_eq!(resolve_hint(&hints, ['a', 'd']), Some(103));
        assert_eq!(resolve_hint(&hints, ['a', 'f']), Some(104));
        assert_eq!(resolve_hint(&hints, ['z', 'z']), None);
    }

    #[test]
    fn test_hint_filtering_by_first_char() {
        let alphabet_len = HINT_CHARS.len();
        // Generate enough refs to roll over into the second first-character ('s')
        let ref_ids: Vec<u32> = (1..=(alphabet_len as u32 + 5)).collect();
        let hints = generate_hints(&ref_ids);

        let filtered_a = filter_hints(&hints, 'a');
        assert_eq!(filtered_a.len(), alphabet_len);

        let filtered_s = filter_hints(&hints, 's');
        assert_eq!(filtered_s.len(), 5);

        let filtered_z = filter_hints(&hints, 'z');
        assert!(filtered_z.is_empty());
    }

    #[test]
    fn test_large_hint_generation_uniqueness() {
        // Up to 19 * 19 = 361 combinations are completely unique
        let max_unique = HINT_CHARS.len() * HINT_CHARS.len();
        let ref_ids: Vec<u32> = (0..max_unique as u32).collect();
        let hints = generate_hints(&ref_ids);

        let mut seen = std::collections::HashSet::new();
        for &chars in hints.values() {
            assert!(seen.insert(chars), "Duplicate hint found: {:?}", chars);
        }
        assert_eq!(seen.len(), max_unique);
    }
}
