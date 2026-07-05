/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Spacing, TokenStream, TokenTree};
use syn::{Expr, ExprLit, Ident, Lit};

/// Parsed token shape for `::Variant => expr` invariant arms.
#[derive(Debug, Clone)]
pub struct VariantInvariantSyntax {
    pub variant_ident: Ident,
    pub tail: TokenStream,
    pub expr: TokenStream,
}

/// Split a contract attribute payload at top-level commas.
pub fn attribute_segments(tokens: TokenStream) -> Vec<TokenStream> {
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Punct(punct)
                if punct.as_char() == ',' && punct.spacing() == Spacing::Alone =>
            {
                if !segment.is_empty() {
                    segments.push(segment.into_iter().collect());
                    segment = Vec::new();
                }
            }
            token => segment.push(token),
        }
    }
    if !segment.is_empty() {
        segments.push(segment.into_iter().collect());
    }
    segments
}

/// Split contract predicates and discard the optional trailing description.
pub fn predicate_segments(tokens: TokenStream) -> Vec<TokenStream> {
    let mut segments = attribute_segments(tokens);
    if segments.last().is_some_and(is_string_literal_segment) {
        segments.pop();
    }
    segments
}

pub fn contract_attribute_is_true_marker(tokens: TokenStream) -> bool {
    let segments = predicate_segments(tokens);
    !segments.is_empty() && segments.into_iter().all(segment_is_true_marker)
}

pub fn segment_is_true_marker(segment: TokenStream) -> bool {
    if let Some(parsed) = parse_variant_invariant_segment(segment.clone()) {
        let Ok(parsed) = parsed else {
            return false;
        };
        return contract_attribute_is_true_marker(parsed.expr);
    }
    syn::parse2::<Expr>(segment).is_ok_and(|expr| expr_is_true_literal(&expr))
}

pub fn expr_is_true_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Lit(ExprLit {
            lit: Lit::Bool(lit),
            ..
        }) if lit.value
    )
}

pub fn is_string_literal_segment(segment: &TokenStream) -> bool {
    syn::parse2::<Expr>(segment.clone()).is_ok_and(|expr| {
        matches!(
            expr,
            Expr::Lit(ExprLit {
                lit: Lit::Str(_),
                ..
            })
        )
    })
}

pub fn parse_variant_invariant_segment(
    segment: TokenStream,
) -> Option<syn::Result<VariantInvariantSyntax>> {
    let tokens = segment.clone().into_iter().collect::<Vec<_>>();
    if !starts_with_double_colon(&tokens) {
        return None;
    }
    let Some(arrow_index) = top_level_fat_arrow_index(&tokens) else {
        return Some(Err(syn::Error::new_spanned(
            segment,
            "enum variant invariant requires `=>`",
        )));
    };

    let Some(TokenTree::Ident(variant_ident)) = tokens.get(2) else {
        return Some(Err(syn::Error::new_spanned(
            segment,
            "enum variant invariant must start with `::Variant`",
        )));
    };

    let tail = tokens[3..arrow_index]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    let expr = tokens[arrow_index + 2..]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    if expr.is_empty() {
        return Some(Err(syn::Error::new_spanned(
            segment,
            "enum variant invariant requires an expression after `=>`",
        )));
    }

    Some(Ok(VariantInvariantSyntax {
        variant_ident: variant_ident.clone(),
        tail,
        expr,
    }))
}

fn starts_with_double_colon(tokens: &[TokenTree]) -> bool {
    matches!(
        (tokens.first(), tokens.get(1)),
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second)))
            if first.as_char() == ':'
                && first.spacing() == Spacing::Joint
                && second.as_char() == ':'
    )
}

fn top_level_fat_arrow_index(tokens: &[TokenTree]) -> Option<usize> {
    tokens.windows(2).position(|window| {
        matches!(
            (&window[0], &window[1]),
            (TokenTree::Punct(first), TokenTree::Punct(second))
                if first.as_char() == '='
                    && first.spacing() == Spacing::Joint
                    && second.as_char() == '>'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn true_marker_allows_trailing_description() {
        assert!(contract_attribute_is_true_marker(quote!(
            true,
            "raw wire marker"
        )));
    }

    #[test]
    fn true_marker_allows_variant_arm_with_description() {
        assert!(contract_attribute_is_true_marker(
            quote!(::Compact(_) => true, "raw wire marker")
        ));
    }

    #[test]
    fn true_marker_rejects_non_true_predicate() {
        assert!(!contract_attribute_is_true_marker(quote!(value > 0)));
        assert!(!contract_attribute_is_true_marker(
            quote!(::Compact(value) => value > 0)
        ));
    }

    #[test]
    fn parses_variant_invariant_shape() {
        let parsed = parse_variant_invariant_segment(quote!(::Pair(left, right) => *right > 0))
            .expect("variant arm")
            .expect("valid shape");

        assert_eq!(parsed.variant_ident.to_string(), "Pair");
        assert_eq!(parsed.tail.to_string(), "(left , right)");
        assert_eq!(parsed.expr.to_string(), "* right > 0");
    }

    #[test]
    fn variant_parse_errors_have_non_empty_diagnostics() {
        for segment in [quote!(::Pair), quote!(:: => value > 0), quote!(::Pair =>)] {
            let error = parse_variant_invariant_segment(segment)
                .expect("variant arm")
                .expect_err("invalid variant invariant shape");
            assert!(!error.to_string().is_empty());
        }
    }
}
