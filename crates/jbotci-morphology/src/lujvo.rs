#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

#[invariant(true)]
#[invariant(::Lujvo => true)]
#[invariant(::Cmevla => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LujvoBuildMode {
    Lujvo,
    Cmevla,
}

#[invariant(!word.is_empty())]
#[invariant(!parts.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoCandidate {
    pub word: String,
    pub parts: Vec<String>,
    pub score: i32,
}

#[requires(true)]
#[ensures(true)]
pub fn choose_best_lujvo_candidate(
    mode: LujvoBuildMode,
    choices: &[Vec<String>],
) -> Option<LujvoCandidate> {
    choose_best_candidate_from(mode, choices, &mut Vec::new(), None)
}

#[requires(true)]
#[ensures(true)]
fn choose_best_candidate_from(
    mode: LujvoBuildMode,
    choices: &[Vec<String>],
    selected: &mut Vec<String>,
    best: Option<LujvoCandidate>,
) -> Option<LujvoCandidate> {
    let Some((next_choices, rest)) = choices.split_first() else {
        let bonded = bond_rafsis(selected)?;
        let word = bonded.concat();
        let candidate = new!(LujvoCandidate {
            score: lujvo_score(&bonded),
            parts: bonded,
            word,
        });
        if mode == LujvoBuildMode::Lujvo && !is_valid_lujvo_candidate(&candidate) {
            return best;
        }
        return Some(select_better_candidate(best, candidate));
    };

    let mut current_best = best;
    for choice in next_choices {
        selected.push(choice.clone());
        current_best = choose_best_candidate_from(mode, rest, selected, current_best);
        selected.pop();
    }
    current_best
}

#[requires(true)]
#[ensures(true)]
fn select_better_candidate(
    current_best: Option<LujvoCandidate>,
    candidate: LujvoCandidate,
) -> LujvoCandidate {
    let Some(current_best) = current_best else {
        return candidate;
    };
    if candidate.score < current_best.score
        || (candidate.score == current_best.score && candidate.word < current_best.word)
    {
        candidate
    } else {
        current_best
    }
}

#[requires(true)]
#[ensures(true)]
pub fn bond_rafsis(rafsis: &[String]) -> Option<Vec<String>> {
    if rafsis.len() < 2 {
        return None;
    }
    let first = rafsis.first()?.clone();
    let second = rafsis.get(1)?;
    let mut bonded = vec![first.clone()];
    if should_insert_cvv_hyphen(&first, second, rafsis.len()) {
        bonded.push(if second.starts_with('r') {
            "n".to_owned()
        } else {
            "r".to_owned()
        });
    }
    for pair in rafsis.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        if let Some(hyphen) = y_hyphen_for_pair(previous, next) {
            bonded.push(hyphen.to_owned());
        }
        bonded.push(next.clone());
    }
    if tosmabru(&bonded) {
        bonded.insert(1, "y".to_owned());
    }
    Some(bonded)
}

#[requires(true)]
#[ensures(ret -> !word_text.is_empty())]
pub fn is_valid_lujvo_candidate_word(word_text: &str) -> bool {
    let Ok(words) = crate::segment_words_with_modifiers(word_text) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    word_like
        .bare_word()
        .is_some_and(|word| word.kind() == crate::WordKind::Lujvo)
}

#[requires(!candidate.word.is_empty())]
#[ensures(ret -> !candidate.parts.is_empty())]
fn is_valid_lujvo_candidate(candidate: &LujvoCandidate) -> bool {
    let Ok(words) = crate::segment_words_with_modifiers(&candidate.word) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    let Some(word) = word_like.bare_word() else {
        return false;
    };
    if word.kind() != crate::WordKind::Lujvo {
        return false;
    }
    word.lujvo_parts()
        .is_some_and(|parts| lujvo_parts_match(parts.as_slice(), &candidate.parts))
}

#[requires(true)]
#[ensures(true)]
fn lujvo_parts_match(actual: &[crate::LujvoPart], expected: &[String]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(part, expected)| crate::canonical_text_eq(part.phonemes().as_str(), expected))
}

#[requires(true)]
#[ensures(true)]
pub fn ensure_cmevla_word(word_text: &str) -> String {
    if is_cmevla(word_text) {
        word_text.to_owned()
    } else {
        format!("{word_text}s")
    }
}

#[requires(true)]
#[ensures(true)]
pub fn ends_with_consonant(word_text: &str) -> bool {
    word_text.chars().last().is_some_and(is_consonant)
}

#[requires(true)]
#[ensures(true)]
pub fn ends_with_vowel(word_text: &str) -> bool {
    word_text.chars().last().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
pub fn is_bonding_hyphen(part: &str) -> bool {
    matches!(part, "y" | "y'" | "r" | "n")
}

#[requires(true)]
#[ensures(true)]
pub fn syllables_pattern(text: &str) -> Option<String> {
    text.chars().map(classify_syllable_char).collect()
}

#[requires(true)]
#[ensures(true)]
pub fn is_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u')
}

#[requires(true)]
#[ensures(true)]
pub fn is_consonant(value: char) -> bool {
    matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmevla(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|value| !matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | '\''))
}

#[requires(true)]
#[ensures(true)]
pub fn permissible_consonant_pair(first: char, second: char) -> Option<i32> {
    let consonant_order = "rlnmbvdgjzscxktfp";
    let first_index = consonant_order.chars().position(|value| value == first)?;
    let second_index = consonant_order.chars().position(|value| value == second)?;
    PAIR_MATRIX
        .get(first_index)
        .and_then(|row| row.get(second_index))
        .copied()
}

#[requires(true)]
#[ensures(true)]
fn classify_syllable_char(value: char) -> Option<char> {
    if is_vowel(value) {
        Some('V')
    } else if is_consonant(value) {
        Some('C')
    } else if value == '\'' {
        Some('\'')
    } else if value == 'y' {
        Some('Y')
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn needs_y_hyphen(previous: &str, next: &str) -> bool {
    let previous_pattern = syllables_pattern(previous);
    let previous_tail = previous.chars().last();
    let next_head = next.chars().next();
    matches!(previous_pattern.as_deref(), Some("CVCC" | "CCVC"))
        || matches!(
            (previous_tail, next_head),
            (Some(left), Some(right))
                if is_consonant(left)
                    && is_consonant(right)
                    && permissible_consonant_pair(left, right) == Some(0)
        )
        || (previous_tail == Some('n')
            && (next.starts_with("ts")
                || next.starts_with("tc")
                || next.starts_with("dz")
                || next.starts_with("dj")))
}

#[requires(true)]
#[ensures(ret.is_none_or(|hyphen| is_bonding_hyphen(hyphen)))]
fn y_hyphen_for_pair(previous: &str, next: &str) -> Option<&'static str> {
    let previous_tail = previous.chars().last();
    let needs_hyphen = needs_y_hyphen(previous, next)
        || previous_tail.is_some_and(is_consonant) && starts_with_vowel_or_glide(next);
    needs_hyphen.then(|| {
        if starts_with_vowel_nucleus_after_y(next) {
            "y'"
        } else {
            "y"
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn starts_with_vowel_or_glide(text: &str) -> bool {
    text.chars().next().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
fn starts_with_vowel_nucleus_after_y(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_vowel(first) {
        return false;
    }
    !matches!(first, 'i' | 'u') || !chars.next().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
fn should_insert_cvv_hyphen(first_rafsi: &str, second: &str, rafsi_count: usize) -> bool {
    matches!(
        syllables_pattern(first_rafsi).as_deref(),
        Some("CVV" | "CV'V")
    ) && (rafsi_count > 2 || syllables_pattern(second).as_deref() != Some("CCV"))
}

#[requires(true)]
#[ensures(true)]
fn tosmabru(parts: &[String]) -> bool {
    let Some(last_part) = parts.last() else {
        return false;
    };
    if is_cmevla(last_part) {
        return false;
    }
    if let Some(y_index) = parts.iter().position(|part| part == "y") {
        let heads = &parts[..y_index];
        return heads.len() > 1
            && heads
                .iter()
                .all(|part| syllables_pattern(part).as_deref() == Some("CVC"))
            && heads
                .windows(2)
                .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
    }
    if syllables_pattern(last_part).as_deref() == Some("CVCCV") {
        let chars = last_part.chars().collect::<Vec<_>>();
        if chars.len() >= 4
            && is_consonant(chars[2])
            && is_consonant(chars[3])
            && permissible_consonant_pair(chars[2], chars[3]) == Some(2)
        {
            let heads = &parts[..parts.len().saturating_sub(1)];
            return !heads.is_empty()
                && heads
                    .iter()
                    .all(|part| syllables_pattern(part).as_deref() == Some("CVC"))
                && parts
                    .windows(2)
                    .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn consonant_pair_is_rank_two(left: &str, right: &str) -> bool {
    matches!(
        (left.chars().last(), right.chars().next()),
        (Some(left_tail), Some(right_head))
            if is_consonant(left_tail)
                && is_consonant(right_head)
                && permissible_consonant_pair(left_tail, right_head) == Some(2)
    )
}

#[requires(true)]
#[ensures(true)]
fn lujvo_score(rafsi_sequence: &[String]) -> i32 {
    let lujvo_text = rafsi_sequence.concat();
    let total_length = lujvo_text.chars().count() as i32;
    let apostrophe_count = lujvo_text.chars().filter(|value| *value == '\'').count() as i32;
    let hyphen_count = rafsi_sequence
        .iter()
        .filter(|part| is_bonding_hyphen(part))
        .count() as i32;
    let rafsi_shape_score = rafsi_sequence
        .iter()
        .filter_map(|part| syllables_pattern(part))
        .map(|pattern| match pattern.as_str() {
            "CVCCV" => 1,
            "CVCC" => 2,
            "CCVCV" => 3,
            "CCVC" => 4,
            "CVC" => 5,
            "CV'V" => 6,
            "CCV" => 7,
            "CVV" => 8,
            _ => 0,
        })
        .sum::<i32>();
    let vowel_count = lujvo_text.chars().filter(|value| is_vowel(*value)).count() as i32;
    1000 * total_length - 500 * apostrophe_count + 100 * hyphen_count
        - 10 * rafsi_shape_score
        - vowel_count
}

const PAIR_MATRIX: [[i32; 17]; 17] = [
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [2, 1, 1, 1, 1, 1, 0, 1, 2, 2, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2],
    [2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1],
    [2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 2, 2, 1, 1, 0, 1, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
];

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bonds_cvv_initial_rafsi_with_r_or_n() {
        assert_eq!(
            bond_rafsis(&["bau".to_owned(), "gri".to_owned(), "kla".to_owned()]),
            Some(vec![
                "bau".to_owned(),
                "r".to_owned(),
                "gri".to_owned(),
                "kla".to_owned()
            ])
        );
        assert_eq!(
            bond_rafsis(&["bau".to_owned(), "rok".to_owned()]),
            Some(vec!["bau".to_owned(), "n".to_owned(), "rok".to_owned()])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inserts_y_for_impermissible_consonant_pair() {
        assert_eq!(
            bond_rafsis(&["jbon".to_owned(), "bau".to_owned()]),
            Some(vec!["jbon".to_owned(), "y".to_owned(), "bau".to_owned()])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strict_lujvo_candidates_need_valid_lujvo_shape() {
        assert!(is_valid_lujvo_candidate_word("jbogri"));
        assert!(is_valid_lujvo_candidate_word("soirsai"));
        assert!(is_valid_lujvo_candidate_word("ro'inre'o"));
        assert!(is_valid_lujvo_candidate_word("jetcybolxada"));
        assert!(!is_valid_lujvo_candidate_word("babau"));
        assert!(!is_valid_lujvo_candidate_word("soisai"));
        assert!(!is_valid_lujvo_candidate_word("xlamkai"));
        assert!(!is_valid_lujvo_candidate_word("xlaglymlu"));
        assert!(!is_valid_lujvo_candidate_word("kerlyu'ukerlo"));
    }
}
