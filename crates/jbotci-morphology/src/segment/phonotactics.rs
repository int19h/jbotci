#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

#[invariant(::Forbidden => true)]
#[invariant(::Permissible => true)]
#[invariant(::Initial => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsonantPairClass {
    Forbidden,
    Permissible,
    Initial,
}

impl ConsonantPairClass {
    #[requires(true)]
    #[ensures(ret == !matches!(self, Self::Forbidden))]
    pub fn is_permissible(self) -> bool {
        !matches!(self, Self::Forbidden)
    }

    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Initial))]
    pub fn is_initial(self) -> bool {
        matches!(self, Self::Initial)
    }
}

#[requires(true)]
#[ensures(ret == consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_initial))]
pub(crate) fn initial_pair_chars(first: char, second: char) -> bool {
    consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_initial)
}

#[requires(true)]
#[ensures(ret == consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_permissible))]
pub(crate) fn permissible_consonant_pair(first: char, second: char) -> bool {
    consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_permissible)
}

#[requires(true)]
#[ensures(ret == (permissible_consonant_pair(first, second) || (first == 'm' && second == 'z')))]
pub(crate) fn experimental_permissible_consonant_pair(first: char, second: char) -> bool {
    permissible_consonant_pair(first, second) || (first == 'm' && second == 'z')
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn consonant_pair_class(first: char, second: char) -> Option<ConsonantPairClass> {
    let first_index = CONSONANT_ORDER.find(first)?;
    let second_index = CONSONANT_ORDER.find(second)?;
    PAIR_MATRIX
        .get(first_index)
        .and_then(|row| row.get(second_index))
        .copied()
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(super) fn consonant_pair_class_for_test(
    first: char,
    second: char,
) -> Option<ConsonantPairClass> {
    consonant_pair_class(first, second)
}

#[cfg(test)]
pub(super) const CONSONANT_ORDER_FOR_TEST: &str = CONSONANT_ORDER;

const CONSONANT_ORDER: &str = "rlnmbvdgjzscxktfp";

use ConsonantPairClass::{Forbidden as F, Initial as I, Permissible as P};

// CLL 3.6 defines permissible consonant pairs; CLL 3.7 defines the smaller
// subset that can begin ordinary words. `Initial` pairs are therefore also
// permissible; `Forbidden` pairs require a hyphen or a word boundary.
const PAIR_MATRIX: [[ConsonantPairClass; 17]; 17] = [
    [F, P, P, P, P, P, P, P, P, P, P, P, P, P, P, P, P],
    [P, F, P, P, P, P, P, P, P, P, P, P, P, P, P, P, P],
    [P, P, F, P, P, P, P, P, P, P, P, P, P, P, P, P, P],
    [I, I, P, F, P, P, P, P, P, F, P, P, P, P, P, P, P],
    [I, I, P, P, F, P, P, P, P, P, F, F, F, F, F, F, F],
    [I, I, P, P, P, F, P, P, P, P, F, F, F, F, F, F, F],
    [I, P, P, P, P, P, F, P, I, I, F, F, F, F, F, F, F],
    [I, I, P, P, P, P, P, F, P, P, F, F, F, F, F, F, F],
    [P, P, P, I, I, I, I, I, F, F, F, F, F, F, F, F, F],
    [P, P, P, I, I, I, I, I, F, F, F, F, F, F, F, F, F],
    [I, I, I, I, F, F, F, F, F, F, F, F, P, I, I, I, I],
    [I, I, I, I, F, F, F, F, F, F, F, F, F, I, I, I, I],
    [I, I, P, P, F, F, F, F, F, F, P, F, F, F, P, P, P],
    [I, I, P, P, F, F, F, F, F, F, P, P, F, F, P, P, P],
    [I, P, P, P, F, F, F, F, F, F, I, I, P, P, F, P, P],
    [I, I, P, P, F, F, F, F, F, F, P, P, P, P, P, F, P],
    [I, I, P, P, F, F, F, F, F, F, P, P, P, P, P, P, F],
];
