use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_semantics::notation::sexpr::datum::{Datum, print_document};
use jbotci_semantics::notation::sexpr::{
    parse_v0_document, parse_v0_expressions, print_v0_document,
};

const FROZEN_SAMPLES: &str = include_str!("../../../docs/smusni/samples.md");
const LEXICAL_REGISTRY: &str = include_str!("../data/smusni-v0/registry/lexical.jsonl");
const SAMPLE_BODY: &str = "(Assert (klama This))";

#[invariant(*ordinal > 0 && !source.is_empty())]
#[derive(Debug)]
struct LispBlock {
    ordinal: usize,
    source: String,
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_frozen_lisp_sample_obeys_the_v0_serialization_grammar() {
    let blocks = lisp_blocks(FROZEN_SAMPLES);
    let supported_roots = LEXICAL_REGISTRY
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).unwrap()["normalized-root"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    let mut sample_roots = BTreeSet::new();
    assert!(
        !blocks.is_empty(),
        "the frozen samples must contain Lisp blocks"
    );

    for block in blocks {
        let source = block.source.replace("⟦body⟧", SAMPLE_BODY);
        if source.trim_start().starts_with("(Smusni") {
            let document = parse_v0_document(&source).unwrap_or_else(|error| {
                panic!(
                    "sample Lisp block {} is not a v0 document: {error}",
                    block.ordinal
                )
            });
            collect_lexical_dependencies(&document.to_datum(), &mut sample_roots);
            let printed = print_v0_document(&document);
            assert_eq!(
                parse_v0_document(&printed),
                Ok(document),
                "sample Lisp block {} did not round-trip",
                block.ordinal,
            );
        } else {
            let expressions = parse_v0_expressions(&source).unwrap_or_else(|error| {
                panic!(
                    "sample Lisp block {} is not a v0 fragment: {error}",
                    block.ordinal
                )
            });
            assert!(
                !expressions.is_empty(),
                "sample Lisp block {} is an empty fragment",
                block.ordinal,
            );
            for expression in expressions {
                collect_lexical_dependencies(&expression.to_datum(), &mut sample_roots);
                let printed = print_document(&expression.to_datum());
                assert_eq!(
                    parse_v0_expressions(&printed),
                    Ok(vec![expression]),
                    "expression in sample Lisp block {} did not round-trip",
                    block.ordinal,
                );
            }
        }
    }

    let unsupported = sample_roots
        .difference(&supported_roots)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        unsupported.is_empty(),
        "frozen samples use lexical roots absent from the v0 registry: {unsupported:?}"
    );
}

#[requires(true)]
#[ensures(!ret || !token.is_empty())]
fn is_ascii_lexical_root(token: &str) -> bool {
    !token.is_empty()
        && token
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || matches!(byte, b'\'' | b'-'))
}

#[requires(true)]
#[ensures(true)]
fn collect_lexical_dependencies(datum: &Datum, roots: &mut BTreeSet<String>) {
    let Some(items) = datum.as_list() else {
        return;
    };
    if let Some(head) = items.first().and_then(Datum::as_atom) {
        if is_ascii_lexical_root(head) {
            roots.insert(head.to_owned());
        }
        if head == "DropPlace"
            && let Some(root) = items.get(1).and_then(Datum::as_atom)
            && is_ascii_lexical_root(root)
        {
            roots.insert(root.to_owned());
        }
    }
    for item in items {
        collect_lexical_dependencies(item, roots);
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|block| !block.source.is_empty()))]
fn lisp_blocks(markdown: &str) -> Vec<LispBlock> {
    let mut blocks = Vec::new();
    let mut current = None::<String>;

    for line in markdown.lines() {
        match (&mut current, line) {
            (None, "```lisp") => current = Some(String::new()),
            (Some(source), "```") => {
                let source = std::mem::take(source);
                blocks.push(new!(LispBlock {
                    ordinal: blocks.len() + 1,
                    source,
                }));
                current = None;
            }
            (Some(source), line) => {
                source.push_str(line);
                source.push('\n');
            }
            (None, _) => {}
        }
    }

    assert!(
        current.is_none(),
        "unterminated Lisp fence in frozen samples"
    );
    blocks
}
