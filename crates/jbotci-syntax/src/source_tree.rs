//! Source-order traversal for syntax-owned AST data.

use bityzba::{contract_trait, requires};
use jbotci_morphology::WordLike;
use smallvec::SmallVec;
use vec1::{Vec1, smallvec_v1::SmallVec1};

use crate::WithIndicators;

#[contract_trait]
pub trait SourceTree {
    #[requires(true)]
    #[ensures(true)]
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>));

    #[requires(true)]
    #[ensures(true)]
    fn source_word_count(&self) -> usize {
        let mut count = 0;
        self.visit_source_words(&mut |_| count += 1);
        count
    }
}

#[requires(true)]
#[ensures(true)]
pub fn source_words<T: SourceTree + ?Sized>(tree: &T) -> Vec<&WithIndicators<WordLike>> {
    let mut words = Vec::new();
    tree.visit_source_words(&mut |word| words.push(word));
    words
}

#[requires(true)]
#[ensures(true)]
pub fn source_spans_are_ordered<T: SourceTree + ?Sized>(tree: &T) -> bool {
    let mut last_end = None;
    let mut ordered = true;
    tree.visit_source_words(&mut |word| {
        if !ordered {
            return;
        }
        for span in word.source_spans() {
            if last_end.is_some_and(|end| end > span.byte_start) {
                ordered = false;
                return;
            }
            last_end = Some(span.byte_end);
        }
    });
    ordered
}

#[contract_trait]
impl SourceTree for WithIndicators<WordLike> {
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        visitor(self);
    }
}

#[contract_trait]
impl<T: SourceTree + ?Sized> SourceTree for Box<T> {
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        (**self).visit_source_words(visitor);
    }
}

#[contract_trait]
impl<T: SourceTree> SourceTree for Option<T> {
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        if let Some(value) = self {
            value.visit_source_words(visitor);
        }
    }
}

#[contract_trait]
impl<T: SourceTree> SourceTree for Vec<T> {
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        for value in self {
            value.visit_source_words(visitor);
        }
    }
}

#[contract_trait]
impl<T: SourceTree> SourceTree for Vec1<T> {
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        for value in self {
            value.visit_source_words(visitor);
        }
    }
}

#[contract_trait]
impl<A> SourceTree for SmallVec<A>
where
    A: smallvec::Array,
    A::Item: SourceTree,
{
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        for value in self {
            value.visit_source_words(visitor);
        }
    }
}

#[contract_trait]
impl<A> SourceTree for SmallVec1<A>
where
    A: smallvec::Array,
    A::Item: SourceTree,
{
    fn visit_source_words<'a>(&'a self, visitor: &mut dyn FnMut(&'a WithIndicators<WordLike>)) {
        for value in self {
            value.visit_source_words(visitor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceTree;
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};

    #[derive(SourceTree)]
    #[invariant(true)]
    struct Pair<T> {
        first: T,
        #[source(skip)]
        ignored: Vec<WithIndicators<WordLike>>,
        second: Option<T>,
    }

    #[derive(SourceTree)]
    #[invariant(true)]
    enum Wrapped<T> {
        One(T),
        Pair { first: T, second: T },
        Empty,
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derive_visits_struct_fields_in_declaration_order() {
        let first = fake_word("mi", 0, 2);
        let ignored = fake_word("cu", 3, 5);
        let second = fake_word("klama", 6, 11);
        let pair = Pair {
            first: first.clone(),
            ignored: vec![ignored],
            second: Some(second.clone()),
        };
        assert_eq!(pair.ignored.len(), 1);

        let words = source_words(&pair);
        assert_eq!(words, vec![&first, &second]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derive_visits_enum_variant_fields_in_declaration_order() {
        let first = fake_word("mi", 0, 2);
        let second = fake_word("klama", 3, 8);
        let wrapped = Wrapped::Pair {
            first: first.clone(),
            second: second.clone(),
        };

        let words = source_words(&wrapped);
        assert_eq!(words, vec![&first, &second]);
        assert_eq!(
            source_words(&Wrapped::<WithIndicators<WordLike>>::Empty).len(),
            0
        );
        assert_eq!(source_words(&Wrapped::One(first.clone())), vec![&first]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordered_span_check_rejects_overlap() {
        let first = fake_word("klama", 3, 8);
        let second = fake_word("mi", 0, 2);
        let pair = Pair {
            first,
            ignored: Vec::new(),
            second: Some(second),
        };

        assert!(!source_spans_are_ordered(&pair));
    }

    #[requires(!text.is_empty())]
    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    fn fake_word(text: &str, byte_start: usize, byte_end: usize) -> WithIndicators<WordLike> {
        use jbotci_morphology::{Word, WordData, WordKind};
        use jbotci_source::SourceSpan;

        WithIndicators::bare(WordLike::bare(Word::from_data(bityzba::data!(Word {
            kind: WordKind::Cmavo,
            phonemes: text.to_owned(),
            span: SourceSpan::new(None, byte_start, byte_end, byte_start, byte_end)
                .expect("valid test span"),
            surface_override: None,
            dialect_transform: None,
        }))))
    }
}
