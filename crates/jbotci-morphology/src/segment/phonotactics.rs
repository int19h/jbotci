#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[requires(true)]
#[ensures(true)]
pub(super) fn initial_pair_chars(first: char, second: char) -> bool {
    matches!(
        (first, second),
        ('b', 'l' | 'r')
            | ('c', 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 't')
            | ('d', 'j' | 'r' | 'z')
            | ('f', 'l' | 'r')
            | ('g', 'l' | 'r')
            | ('j', 'b' | 'd' | 'g' | 'm' | 'v')
            | ('k', 'l' | 'r')
            | ('m', 'l' | 'r')
            | ('p', 'l' | 'r')
            | ('s', 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 't')
            | ('t', 'c' | 'r' | 's')
            | ('v', 'l' | 'r')
            | ('x', 'l' | 'r')
            | ('z', 'b' | 'd' | 'g' | 'm' | 'v')
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn permissible_consonant_pair(first: char, second: char) -> bool {
    matches!(consonant_pair_class(first, second), Some(1 | 2))
}

#[requires(true)]
#[ensures(ret == (permissible_consonant_pair(first, second) || (first == 'm' && second == 'z')))]
pub(super) fn experimental_permissible_consonant_pair(first: char, second: char) -> bool {
    permissible_consonant_pair(first, second) || (first == 'm' && second == 'z')
}

#[requires(true)]
#[ensures(true)]
fn consonant_pair_class(first: char, second: char) -> Option<u8> {
    let first_index = CONSONANT_ORDER.find(first)?;
    let second_index = CONSONANT_ORDER.find(second)?;
    PAIR_MATRIX
        .get(first_index)
        .and_then(|row| row.get(second_index))
        .copied()
}

const CONSONANT_ORDER: &str = "rlnmbvdgjzscxktfp";

const PAIR_MATRIX: [[u8; 17]; 17] = [
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
