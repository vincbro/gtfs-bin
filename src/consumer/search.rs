use crate::{consumer::Consumer, models::SearchIndex};

/// Lowercases text and strips accents, so `Örebro` and `orebro` compare equal.
///
/// It covers the Latin letters with accents that European place names use. Anything else is only
/// lowercased.
#[must_use]
pub(super) fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for character in text.chars().flat_map(char::to_lowercase) {
        match character {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => folded.push('a'),
            'ç' | 'ć' | 'č' | 'ĉ' | 'ċ' => folded.push('c'),
            'ď' | 'đ' => folded.push('d'),
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => folded.push('e'),
            'ĝ' | 'ğ' | 'ġ' | 'ģ' => folded.push('g'),
            'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => folded.push('i'),
            'ł' | 'ĺ' | 'ľ' | 'ļ' => folded.push('l'),
            'ñ' | 'ń' | 'ň' | 'ņ' => folded.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => folded.push('o'),
            'ř' | 'ŕ' => folded.push('r'),
            'ś' | 'š' | 'ş' | 'ŝ' => folded.push('s'),
            'ť' | 'ţ' => folded.push('t'),
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' | 'ų' => folded.push('u'),
            'ý' | 'ÿ' => folded.push('y'),
            'ź' | 'ż' | 'ž' => folded.push('z'),
            'æ' => folded.push_str("ae"),
            'œ' => folded.push_str("oe"),
            'ß' => folded.push_str("ss"),
            // Combining accent marks, for text that wasn't composed.
            '\u{0300}'..='\u{036f}' => {}
            other => folded.push(other),
        }
    }
    folded
}

impl Consumer<'_> {
    /// Replaces the contents of `found` with the named places that match `query`, best first.
    ///
    /// Case and accents are ignored. A name that equals the query comes first, then names that
    /// start with it, then names with a word that starts with it, then names that contain it.
    /// An empty query finds nothing.
    ///
    /// It reads every name, which is fine for a search box and not for a loop.
    pub fn search_by_name(&self, query: &str, found: &mut Vec<SearchIndex>) {
        found.clear();
        let query = fold(query.trim());
        if query.is_empty() {
            return;
        }

        let mut ranked: Vec<(u8, SearchIndex)> = Vec::new();
        for search in self.search_stops {
            let name = fold(self.string(search.name));
            let rank = if name == query {
                0
            } else if name.starts_with(&query) {
                1
            } else if name.split(' ').any(|word| word.starts_with(&query)) {
                2
            } else if name.contains(&query) {
                3
            } else {
                continue;
            };
            ranked.push((rank, search.index));
        }
        ranked.sort_unstable();
        found.extend(ranked.into_iter().map(|(_, index)| index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swedish_letters_fold_to_plain_ones() {
        assert_eq!(fold("Örebro Södra"), "orebro sodra", "ö becomes o");
        assert_eq!(fold("Västerås"), "vasteras", "ä and å become a");
    }

    #[test]
    fn other_accents_and_ligatures_fold() {
        assert_eq!(fold("Zürich Straße"), "zurich strasse", "ü and ß");
        assert_eq!(fold("Łódź"), "lodz", "polish letters");
    }

    #[test]
    fn combining_marks_are_dropped() {
        assert_eq!(
            fold("e\u{301}cole"),
            "ecole",
            "an accent written as a separate mark"
        );
    }
}
