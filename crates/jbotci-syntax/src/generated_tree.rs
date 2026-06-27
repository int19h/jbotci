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

    node generated_item_node -> GeneratedItemSyntax {
        fields {
            field token = cmavo(Be);
        }
    }

    node generated_pair_node(generated_item) -> GeneratedPairSyntax {
        fields {
            field head = cmavo(Be);
            field nonempty: Vec<Token> = many1(cmavo(Be));
            require cmavo(Bo).not();
            scratch parser_only = cmavo(Bo).ignored();
            #[tree_child(primary)]
            field child = boxed(generated_item);
            default trailing: Vec<Token> = Vec::new();
        }
    }

    node generated_choice_first -> GeneratedChoiceSyntax {
        construct variant First;
        fields {
            field token = cmavo(Be);
        }
    }

    node generated_choice_second(generated_item) -> GeneratedChoiceSyntax {
        construct variant Second;
        fields {
            #[tree_child(primary)]
            field item = boxed(generated_item);
        }
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
        let item = GeneratedItemSyntax {
            token: token.clone(),
        };
        let pair = GeneratedPairSyntax {
            head: token.clone(),
            nonempty: vec![token.clone()],
            child: Box::new(item),
            trailing: vec![token],
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
