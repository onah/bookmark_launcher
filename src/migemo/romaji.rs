// Hiragana / katakana / romaji-mora conversion.
//
// A "mora" here is the romaji spelling of one Japanese sound unit (e.g. "ge", "n", "kya").
// Item text is converted hiragana/katakana -> mora list at index time (add_item).
// User queries are converted ascii romaji -> mora list at search time (query_to_morae).
// Both use the same canonical spellings so a Vec<String> equality check finds matches.

/// Convert katakana characters to their hiragana equivalents; everything else is untouched.
pub fn katakana_to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{30A1}'..='\u{30F6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            other => other,
        })
        .collect()
}

/// Convert a hiragana string into its mora list, expanding sokuon (っ) and
/// the long vowel mark (ー) along the way.
pub fn hiragana_to_morae(s: &str) -> Vec<String> {
    hiragana_to_morae_with_spans(s)
        .into_iter()
        .map(|(_, _, mora)| mora)
        .collect()
}

/// Same as [`hiragana_to_morae`] but also returns, for each mora, the char
/// range (in `s`) of the kana that produced it. Used to map a matched mora
/// back to the original text for highlighting.
pub fn hiragana_to_morae_with_spans(s: &str) -> Vec<(usize, usize, String)> {
    let chars: Vec<char> = s.chars().collect();
    let mut result: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == 'ー' {
            if let Some((_, _, last)) = result.last() {
                let vowel = last_vowel(last);
                result.push((i, i + 1, vowel.to_string()));
            }
            i += 1;
            continue;
        }

        if c == 'っ' {
            if i + 1 < chars.len() {
                let (unit_len, unit_mora) = next_hiragana_unit(&chars, i + 1);
                if let Some(base) = unit_mora {
                    result.push((i, i + 1 + unit_len, double_leading_consonant(&base)));
                    i += 1 + unit_len;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        let (unit_len, unit_mora) = next_hiragana_unit(&chars, i);
        if let Some(m) = unit_mora {
            result.push((i, i + unit_len, m));
        }
        i += unit_len;
    }

    result
}

/// Read one kana unit (a youon digraph or a single kana) starting at `i`.
/// Returns the number of chars consumed and its romaji, if known.
fn next_hiragana_unit(chars: &[char], i: usize) -> (usize, Option<String>) {
    if i + 1 < chars.len() {
        let combo: String = chars[i..i + 2].iter().collect();
        if let Some(r) = youon_mora(&combo) {
            return (2, Some(r.to_string()));
        }
    }
    (1, single_mora(chars[i]).map(|s| s.to_string()))
}

fn double_leading_consonant(base: &str) -> String {
    match base.chars().next() {
        Some(c) => format!("{c}{base}"),
        None => base.to_string(),
    }
}

fn last_vowel(mora: &str) -> char {
    match mora.chars().last() {
        Some(c) if "aiueo".contains(c) => c,
        Some(c) => c,
        None => 'a',
    }
}

fn single_mora(c: char) -> Option<&'static str> {
    Some(match c {
        'あ' => "a",
        'い' => "i",
        'う' => "u",
        'え' => "e",
        'お' => "o",
        'か' => "ka",
        'き' => "ki",
        'く' => "ku",
        'け' => "ke",
        'こ' => "ko",
        'さ' => "sa",
        'し' => "shi",
        'す' => "su",
        'せ' => "se",
        'そ' => "so",
        'た' => "ta",
        'ち' => "chi",
        'つ' => "tsu",
        'て' => "te",
        'と' => "to",
        'な' => "na",
        'に' => "ni",
        'ぬ' => "nu",
        'ね' => "ne",
        'の' => "no",
        'は' => "ha",
        'ひ' => "hi",
        'ふ' => "fu",
        'へ' => "he",
        'ほ' => "ho",
        'ま' => "ma",
        'み' => "mi",
        'む' => "mu",
        'め' => "me",
        'も' => "mo",
        'や' => "ya",
        'ゆ' => "yu",
        'よ' => "yo",
        'ら' => "ra",
        'り' => "ri",
        'る' => "ru",
        'れ' => "re",
        'ろ' => "ro",
        'わ' => "wa",
        'を' => "wo",
        'ん' => "n",
        'が' => "ga",
        'ぎ' => "gi",
        'ぐ' => "gu",
        'げ' => "ge",
        'ご' => "go",
        'ざ' => "za",
        'じ' => "ji",
        'ず' => "zu",
        'ぜ' => "ze",
        'ぞ' => "zo",
        'だ' => "da",
        'ぢ' => "ji",
        'づ' => "zu",
        'で' => "de",
        'ど' => "do",
        'ば' => "ba",
        'び' => "bi",
        'ぶ' => "bu",
        'べ' => "be",
        'ぼ' => "bo",
        'ぱ' => "pa",
        'ぴ' => "pi",
        'ぷ' => "pu",
        'ぺ' => "pe",
        'ぽ' => "po",
        'ゔ' => "vu",
        'ぁ' => "a",
        'ぃ' => "i",
        'ぅ' => "u",
        'ぇ' => "e",
        'ぉ' => "o",
        'ゃ' => "ya",
        'ゅ' => "yu",
        'ょ' => "yo",
        'ゎ' => "wa",
        _ => return None,
    })
}

fn youon_mora(combo: &str) -> Option<&'static str> {
    Some(match combo {
        "きゃ" => "kya",
        "きゅ" => "kyu",
        "きょ" => "kyo",
        "ぎゃ" => "gya",
        "ぎゅ" => "gyu",
        "ぎょ" => "gyo",
        "しゃ" => "sha",
        "しゅ" => "shu",
        "しょ" => "sho",
        "じゃ" => "ja",
        "じゅ" => "ju",
        "じょ" => "jo",
        "ちゃ" => "cha",
        "ちゅ" => "chu",
        "ちょ" => "cho",
        "ぢゃ" => "ja",
        "ぢゅ" => "ju",
        "ぢょ" => "jo",
        "にゃ" => "nya",
        "にゅ" => "nyu",
        "にょ" => "nyo",
        "ひゃ" => "hya",
        "ひゅ" => "hyu",
        "ひょ" => "hyo",
        "びゃ" => "bya",
        "びゅ" => "byu",
        "びょ" => "byo",
        "ぴゃ" => "pya",
        "ぴゅ" => "pyu",
        "ぴょ" => "pyo",
        "みゃ" => "mya",
        "みゅ" => "myu",
        "みょ" => "myo",
        "りゃ" => "rya",
        "りゅ" => "ryu",
        "りょ" => "ryo",
        "ふぁ" => "fa",
        "ふぃ" => "fi",
        "ふぇ" => "fe",
        "ふぉ" => "fo",
        "うぃ" => "wi",
        "うぇ" => "we",
        "うぉ" => "wo",
        "ゔぁ" => "va",
        "ゔぃ" => "vi",
        "ゔぇ" => "ve",
        "ゔぉ" => "vo",
        "てぃ" => "ti",
        "でぃ" => "di",
        "とぅ" => "tu",
        "どぅ" => "du",
        "つぁ" => "tsa",
        "つぃ" => "tsi",
        "つぇ" => "tse",
        "つぉ" => "tso",
        "しぇ" => "she",
        "じぇ" => "je",
        "ちぇ" => "che",
        "くぁ" => "kwa",
        "くぃ" => "kwi",
        "くぇ" => "kwe",
        "くぉ" => "kwo",
        _ => return None,
    })
}

/// Look up a plain (non-doubled) romaji token, normalizing common spelling
/// fluctuations (si -> shi, ti -> chi, ...) to the canonical mora used by
/// [`hiragana_to_morae`].
fn lookup_plain_token(s: &str) -> Option<&'static str> {
    Some(match s {
        "a" => "a",
        "i" => "i",
        "u" => "u",
        "e" => "e",
        "o" => "o",
        "n" => "n",
        "ka" => "ka",
        "ki" => "ki",
        "ku" => "ku",
        "ke" => "ke",
        "ko" => "ko",
        "sa" => "sa",
        "shi" => "shi",
        "si" => "shi",
        "su" => "su",
        "se" => "se",
        "so" => "so",
        "ta" => "ta",
        "chi" => "chi",
        "ti" => "chi",
        "tsu" => "tsu",
        "tu" => "tsu",
        "te" => "te",
        "to" => "to",
        "na" => "na",
        "ni" => "ni",
        "nu" => "nu",
        "ne" => "ne",
        "no" => "no",
        "ha" => "ha",
        "hi" => "hi",
        "fu" => "fu",
        "hu" => "fu",
        "he" => "he",
        "ho" => "ho",
        "ma" => "ma",
        "mi" => "mi",
        "mu" => "mu",
        "me" => "me",
        "mo" => "mo",
        "ya" => "ya",
        "yu" => "yu",
        "yo" => "yo",
        "ra" => "ra",
        "ri" => "ri",
        "ru" => "ru",
        "re" => "re",
        "ro" => "ro",
        "wa" => "wa",
        "wo" => "wo",
        "ga" => "ga",
        "gi" => "gi",
        "gu" => "gu",
        "ge" => "ge",
        "go" => "go",
        "za" => "za",
        "ji" => "ji",
        "zi" => "ji",
        "zu" => "zu",
        "ze" => "ze",
        "zo" => "zo",
        "da" => "da",
        "di" => "ji",
        "de" => "de",
        "do" => "do",
        "ba" => "ba",
        "bi" => "bi",
        "bu" => "bu",
        "be" => "be",
        "bo" => "bo",
        "pa" => "pa",
        "pi" => "pi",
        "pu" => "pu",
        "pe" => "pe",
        "po" => "po",
        "vu" => "vu",
        "kya" => "kya",
        "kyu" => "kyu",
        "kyo" => "kyo",
        "gya" => "gya",
        "gyu" => "gyu",
        "gyo" => "gyo",
        "sha" => "sha",
        "shu" => "shu",
        "sho" => "sho",
        "ja" => "ja",
        "ju" => "ju",
        "jo" => "jo",
        "cha" => "cha",
        "chu" => "chu",
        "cho" => "cho",
        "nya" => "nya",
        "nyu" => "nyu",
        "nyo" => "nyo",
        "hya" => "hya",
        "hyu" => "hyu",
        "hyo" => "hyo",
        "bya" => "bya",
        "byu" => "byu",
        "byo" => "byo",
        "pya" => "pya",
        "pyu" => "pyu",
        "pyo" => "pyo",
        "mya" => "mya",
        "myu" => "myu",
        "myo" => "myo",
        "rya" => "rya",
        "ryu" => "ryu",
        "ryo" => "ryo",
        "fa" => "fa",
        "fi" => "fi",
        "fe" => "fe",
        "fo" => "fo",
        "wi" => "wi",
        "we" => "we",
        "va" => "va",
        "vi" => "vi",
        "ve" => "ve",
        "vo" => "vo",
        "du" => "du",
        "tsa" => "tsa",
        "tsi" => "tsi",
        "tse" => "tse",
        "tso" => "tso",
        "she" => "she",
        "je" => "je",
        "che" => "che",
        "kwa" => "kwa",
        "kwi" => "kwi",
        "kwe" => "kwe",
        "kwo" => "kwo",
        _ => return None,
    })
}

/// Try to match one mora token at `chars[pos..]`, either a plain token (up to 4
/// chars, longest match first) or a sokuon-doubled consonant (e.g. "kka").
/// Returns the number of chars consumed and the canonical mora string.
fn match_token(chars: &[char], pos: usize) -> Option<(usize, String)> {
    for len in [4usize, 3, 2, 1] {
        if pos + len <= chars.len() {
            let s: String = chars[pos..pos + len].iter().collect();
            if let Some(canon) = lookup_plain_token(&s) {
                return Some((len, canon.to_string()));
            }
        }
    }

    if pos + 1 < chars.len() && chars[pos] == chars[pos + 1] && chars[pos] != 'n' {
        for len in [3usize, 2, 1] {
            if pos + 1 + len <= chars.len() {
                let s: String = chars[pos + 1..pos + 1 + len].iter().collect();
                if let Some(canon) = lookup_plain_token(&s) {
                    let doubled = double_leading_consonant(canon);
                    return Some((1 + len, doubled));
                }
            }
        }
    }

    None
}

/// Decompose a user-typed romaji query into a mora list, stopping (dropping the
/// rest) as soon as an unrecognized/incomplete fragment is hit. For example
/// "geng" -> ["ge", "n"], the trailing "g" is dropped rather than guessed at.
pub fn query_to_morae(query: &str) -> Vec<String> {
    let chars: Vec<char> = query.to_lowercase().chars().collect();
    let mut morae = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match match_token(&chars, i) {
            Some((len, mora)) => {
                morae.push(mora);
                i += len;
            }
            None => break,
        }
    }

    morae
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_basic_word() {
        assert_eq!(hiragana_to_morae("げんご"), vec!["ge", "n", "go"]);
    }

    #[test]
    fn converts_sokuon() {
        assert_eq!(hiragana_to_morae("がっこう"), vec!["ga", "kko", "u"]);
    }

    #[test]
    fn converts_chouon() {
        assert_eq!(
            hiragana_to_morae("そーすこーど"),
            vec!["so", "o", "su", "ko", "o", "do"]
        );
    }

    #[test]
    fn katakana_run_converts_via_hiragana() {
        let hira = katakana_to_hiragana("ソースコード");
        assert_eq!(
            hiragana_to_morae(&hira),
            vec!["so", "o", "su", "ko", "o", "do"]
        );
    }

    #[test]
    fn query_full_word() {
        assert_eq!(query_to_morae("gengo"), vec!["ge", "n", "go"]);
    }

    #[test]
    fn query_incomplete_trailing_mora_is_dropped() {
        assert_eq!(query_to_morae("geng"), vec!["ge", "n"]);
    }

    #[test]
    fn query_word_final_n_is_confirmed() {
        assert_eq!(query_to_morae("gen"), vec!["ge", "n"]);
    }

    #[test]
    fn query_normalizes_spelling_fluctuation() {
        assert_eq!(query_to_morae("si"), vec!["shi"]);
        assert_eq!(query_to_morae("ti"), vec!["chi"]);
    }

    #[test]
    fn query_kanri() {
        assert_eq!(query_to_morae("kanri"), vec!["ka", "n", "ri"]);
    }
}
