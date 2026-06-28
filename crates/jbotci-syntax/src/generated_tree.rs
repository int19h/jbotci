//! Parallel generated syntax tree model scaffolding.
//!
//! This module is intentionally not wired into production parsing yet. It gives
//! the syntax grammar macro a real in-crate target for grammar-owned valid and
//! recovered AST generation while the public AST remains available to semantics.

#![allow(dead_code)]

pub use crate::tree::RecoveryTreeItem;

jbotci_syntax_macros::syntax_grammar! {
    tree_model {
        #![tree_recovered]

        pub type Token = crate::tree::Token;
    }
    model;

    recursive {
        generated_item: GeneratedItemSyntax;
    }

    alias "generated item" generated_item_alias = generated_item;

    rule "generated item" generated_item -> struct {
        field token <- cmavo(Be);
    }

    rule "generated pair" generated_pair(generated_item) -> struct {
        field head <- cmavo(Be);
        field nonempty <- [one_or_more cmavo(Be)];
        assert !cmavo(Bo);
        #[tree_child(primary)]
        field child <- boxed(generated_item);
    }

    rule "generated choice" generated_choice -> enum {
        generated_choice_first,
        generated_choice_second,
    }

    rule "generated choice" generated_choice_first -> struct {
        field token <- cmavo(Be);
    }

    rule "generated choice" generated_choice_second(generated_item) -> struct {
        #[tree_child(primary)]
        field item <- boxed(generated_item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn generated_model_recovered_round_trip_uses_real_token_type() {
        let token = sample_token("be");
        let item = GeneratedItemSyntax(token.clone());
        let pair = GeneratedPairSyntax {
            head: token.clone(),
            nonempty: vec1::Vec1::new(token.clone()),
            child: Box::new(item),
        };

        let recovered = recovered::GeneratedPairSyntax::from_valid(pair.clone());
        let valid = recovered.try_into_valid().expect("valid recovered tree");
        assert_eq!(valid, pair);
    }

    #[bityzba::requires(!text.is_empty())]
    #[bityzba::ensures(true)]
    fn sample_token(text: &str) -> Token {
        let mut words = jbotci_morphology::segment_words_with_modifiers(text)
            .expect("sample token has valid morphology");
        assert_eq!(words.len(), 1);
        Token::bare(words.remove(0))
    }
}
