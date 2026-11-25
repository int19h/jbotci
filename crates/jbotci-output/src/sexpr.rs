use crate::budget::{ByteLimit, OutputBudget};
use bityzba::{invariant, new, requires};
use std::io::Write;

use crate::{
    BracketRenderOptions, BracketSourceConstruct, BracketSourceFragment, BracketSourceFragmentRole,
    BracketSourceRange,
};

#[invariant(true)]
#[invariant(::Normal => true)]
#[invariant(::Elided => true)]
#[invariant(::Error => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafRole {
    Normal,
    Elided,
    Error,
}

#[invariant(true)]
#[invariant(::Leaf { .. } => true)]
#[invariant(::Node { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SExpr {
    Leaf {
        text: String,
        range: Option<BracketSourceRange>,
        role: LeafRole,
    },
    Node {
        children: Vec<SExpr>,
        range: Option<BracketSourceRange>,
        constructs: Vec<BracketSourceConstruct>,
    },
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.is_empty()))]
pub(crate) fn empty_node() -> SExpr {
    SExpr::Node {
        children: Vec::new(),
        range: None,
        constructs: Vec::new(),
    }
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node(children: Vec<SExpr>) -> SExpr {
    node_with_constructs(children, Vec::new())
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node_with_constructs(
    children: Vec<SExpr>,
    constructs: Vec<BracketSourceConstruct>,
) -> SExpr {
    let mut node_children = Vec::new();
    for child in children {
        match child {
            other if !is_empty(&other) => node_children.push(other),
            _ => {}
        }
    }
    let range = union_child_ranges(&node_children);
    SExpr::Node {
        children: node_children,
        range,
        constructs,
    }
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn leaf(text: String) -> SExpr {
    leaf_with_range(text, None)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    leaf_with_range_and_role(text, range, LeafRole::Normal)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn elided_leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    leaf_with_range_and_role(text, range, LeafRole::Elided)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { role: LeafRole::Error, .. }))]
pub(crate) fn error_leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    SExpr::Leaf {
        text,
        range,
        role: LeafRole::Error,
    }
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
fn leaf_with_range_and_role(
    text: String,
    range: Option<BracketSourceRange>,
    role: LeafRole,
) -> SExpr {
    if text.is_empty() {
        empty_node()
    } else {
        SExpr::Leaf { text, range, role }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_empty(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf { text, role, .. } => *role != LeafRole::Error && text.is_empty(),
        SExpr::Node { children, .. } => children.is_empty(),
    }
}

#[invariant(true)]
struct FlattenFrame {
    remaining: Vec<SExpr>,
    range: Option<BracketSourceRange>,
    constructs: Vec<BracketSourceConstruct>,
    flattened: Vec<SExpr>,
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn flatten(expr: SExpr) -> SExpr {
    let mut frames = Vec::new();
    let mut next = Some(expr);
    let mut completed = None;
    loop {
        if let Some(expr) = next.take() {
            match expr {
                SExpr::Leaf { text, range, role } => {
                    completed = Some(SExpr::Leaf { text, range, role });
                }
                SExpr::Node {
                    mut children,
                    range,
                    constructs,
                } => {
                    children.reverse();
                    frames.push(FlattenFrame {
                        remaining: children,
                        range,
                        constructs,
                        flattened: Vec::new(),
                    });
                    continue;
                }
            }
        }

        if let Some(value) = completed.take() {
            if let Some(mut parent) = frames.pop() {
                if !is_empty(&value) {
                    parent.flattened.push(value);
                }
                frames.push(parent);
            } else {
                return value;
            }
        }

        let Some(mut frame) = frames.pop() else {
            panic!("S-expression flatten traversal lost its root frame");
        };
        if let Some(child) = frame.remaining.pop() {
            frames.push(frame);
            next = Some(child);
            continue;
        }

        let FlattenFrame {
            remaining: _,
            range,
            constructs,
            flattened,
        } = frame;
        let mut flattened = flattened;
        completed = Some(if flattened.len() == 1 {
            attach_constructs(flattened.remove(0), constructs)
        } else {
            SExpr::Node {
                children: flattened,
                range,
                constructs,
            }
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn attach_constructs(mut expr: SExpr, inherited: Vec<BracketSourceConstruct>) -> SExpr {
    let SExpr::Node { constructs, .. } = &mut expr else {
        return expr;
    };
    for construct in inherited {
        if !constructs.contains(&construct) {
            constructs.push(construct);
        }
    }
    expr
}

/// Whether any leaf in the expression marks a recovery error (‼…‼).
///
/// Pending nodes sit on a heap stack. One native frame per nesting level
/// overflows the default thread on a deep `lo nu` chain.
#[requires(true)]
#[ensures(true)]
pub(crate) fn contains_error_leaf(expr: &SExpr) -> bool {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match expr {
            SExpr::Leaf { role, .. } => {
                if *role == LeafRole::Error {
                    return true;
                }
            }
            SExpr::Node { children, .. } => pending.extend(children.iter().rev()),
        }
    }
    false
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub(crate) fn render_bracketed_with_limit(
    expr: &SExpr,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, crate::OutputError> {
    cancellation
        .check()
        .map_err(|_| crate::OutputError::Interrupted)?;
    let mut output = OutputBudget::new(limit).writer();
    write_bracketed(&mut output, 0, expr, options, cancellation).map_err(|error| {
        if error.kind() == std::io::ErrorKind::Interrupted {
            crate::OutputError::Interrupted
        } else {
            crate::OutputError::from_render_io(error, limit)
        }
    })?;
    Ok(output.into_string().expect("bracket output is UTF-8"))
}

#[requires(true)]
#[ensures(true)]
fn has_text(expr: &SExpr) -> bool {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match expr {
            SExpr::Leaf { text, role, .. } => {
                if !text.is_empty() || *role == LeafRole::Error {
                    return true;
                }
            }
            SExpr::Node { children, .. } => pending.extend(children.iter().rev()),
        }
    }
    false
}

#[invariant(true)]
#[invariant(::Node { .. } => true)]
#[invariant(::Space => true)]
#[invariant(::Close { .. } => true)]
enum BracketWriteStep<'a> {
    Node { depth: usize, expr: &'a SExpr },
    Space,
    Close { depth: usize },
}

#[requires(true)]
#[ensures(true)]
fn write_bracketed(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> std::io::Result<()> {
    // One native frame per nesting level overflows the default thread on a
    // deep `lo nu` chain. The pending steps sit on a heap stack.
    let mut steps = vec![BracketWriteStep::Node { depth, expr }];
    while let Some(step) = steps.pop() {
        match step {
            BracketWriteStep::Space => output.write_all(b" ")?,
            BracketWriteStep::Close { depth } => {
                if options.insert_hair_space {
                    output.write_all("\u{200a}".as_bytes())?;
                }
                let (_, close) = bracket_pair(depth);
                write!(output, "{close}")?;
                write_style_close(output, depth, options, LeafRole::Normal)?;
            }
            BracketWriteStep::Node { depth, expr } => {
                // Stop before writing another node. The bytes already written
                // are discarded by the caller on Interrupted.
                if cancellation.is_cancelled() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        jbotci_interrupt::Interrupted,
                    ));
                }
                match expr {
                    SExpr::Leaf { text, role, .. } => {
                        if text.is_empty() && *role != LeafRole::Error {
                            continue;
                        }
                        write_style_open(output, depth, options, *role)?;
                        output.write_all(text.as_bytes())?;
                        write_style_close(output, depth, options, *role)?;
                    }
                    SExpr::Node { children, .. } => {
                        let kids = children
                            .iter()
                            .filter(|child| has_text(child))
                            .collect::<Vec<_>>();
                        let Some((first, rest)) = kids.split_first() else {
                            continue;
                        };
                        if rest.is_empty() {
                            steps.push(BracketWriteStep::Node {
                                depth: depth + 1,
                                expr: *first,
                            });
                            continue;
                        }
                        write_style_open(output, depth, options, LeafRole::Normal)?;
                        let (open, _) = bracket_pair(depth);
                        write!(output, "{open}")?;
                        if options.insert_hair_space {
                            output.write_all("\u{200a}".as_bytes())?;
                        }
                        steps.push(BracketWriteStep::Close { depth });
                        for child in rest.iter().rev() {
                            steps.push(BracketWriteStep::Node {
                                depth: depth + 1,
                                expr: *child,
                            });
                            steps.push(BracketWriteStep::Space);
                        }
                        steps.push(BracketWriteStep::Node {
                            depth: depth + 1,
                            expr: *first,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn write_style_open(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    options: BracketRenderOptions,
    role: LeafRole,
) -> std::io::Result<()> {
    if role == LeafRole::Error {
        if options.color {
            output.write_all(b"\x1b[91m")?;
        }
        output.write_all("‼".as_bytes())?;
        if options.color {
            output.write_all(b"\x1b[9m")?;
        }
    } else if options.color {
        output.write_all(ansi_color_for_depth(depth).as_bytes())?;
        if role == LeafRole::Elided {
            output.write_all(b"\x1b[3m")?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn write_style_close(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    options: BracketRenderOptions,
    role: LeafRole,
) -> std::io::Result<()> {
    if role == LeafRole::Error {
        if options.color {
            output.write_all(b"\x1b[29m")?;
        }
        output.write_all("‼".as_bytes())?;
    } else if role == LeafRole::Elided && options.color {
        output.write_all(b"\x1b[23m")?;
    }
    if options.color {
        output.write_all(ansi_parent_color_for_depth(depth).as_bytes())?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_with_options(expr: &SExpr, options: BracketRenderOptions) -> String {
    render_bracketed_with_limit(
        expr,
        options,
        ByteLimit::new(usize::MAX).expect("positive limit"),
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("unrestricted bracket rendering")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_source_fragments_with_options(
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    render_source_fragments_at_depth(0, expr, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> String {
    let mut output =
        OutputBudget::new(ByteLimit::new(usize::MAX).expect("positive limit")).writer();
    write_bracketed(
        &mut output,
        depth,
        expr,
        options,
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("unrestricted bracket rendering");
    output.into_string().expect("bracket output is UTF-8")
}

#[requires(true)]
#[ensures(true)]
fn render_source_fragments_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    // One native frame per nesting level overflows the default thread on a
    // deep `lo nu` chain. Child results sit on a heap stack.
    let mut jobs = vec![FragmentJob::Enter { depth, expr }];
    let mut results: Vec<Vec<BracketSourceFragment>> = Vec::new();
    while let Some(job) = jobs.pop() {
        match job {
            FragmentJob::Enter { depth, expr } => match expr {
                SExpr::Leaf { text, range, role } => {
                    results.push(vec![BracketSourceFragment::Text {
                        text: if *role == LeafRole::Error {
                            format!("‼{text}‼")
                        } else {
                            text.clone()
                        },
                        range: *range,
                        role: match role {
                            LeafRole::Normal => BracketSourceFragmentRole::Normal,
                            LeafRole::Elided => BracketSourceFragmentRole::Elided,
                            LeafRole::Error => BracketSourceFragmentRole::Error,
                        },
                    }]);
                }
                SExpr::Node { children, .. } => {
                    jobs.push(FragmentJob::Finish {
                        depth,
                        expr,
                        child_count: children.len(),
                    });
                    for child in children.iter().rev() {
                        jobs.push(FragmentJob::Enter {
                            depth: depth + 1,
                            expr: child,
                        });
                    }
                }
            },
            FragmentJob::Finish {
                depth,
                expr,
                child_count,
            } => {
                let start = results.len() - child_count;
                let mut rendered = Vec::new();
                for child_fragments in results.drain(start..) {
                    rendered.extend(
                        child_fragments
                            .into_iter()
                            .filter(|fragment| !source_fragment_is_empty(fragment)),
                    );
                }
                let SExpr::Node {
                    range, constructs, ..
                } = expr
                else {
                    unreachable!("fragment finish runs on a node");
                };
                results.push(match rendered.as_slice() {
                    [] => Vec::new(),
                    [single] => vec![single.clone()],
                    _ => {
                        let (open, close) = bracket_pair(depth);
                        let hair_space = if options.insert_hair_space {
                            "\u{200a}"
                        } else {
                            ""
                        };
                        let mut children = Vec::new();
                        children.push(BracketSourceFragment::Text {
                            text: format!("{open}{hair_space}"),
                            range: *range,
                            role: BracketSourceFragmentRole::Normal,
                        });
                        for (index, fragment) in rendered.into_iter().enumerate() {
                            if index > 0 {
                                children.push(BracketSourceFragment::Text {
                                    text: " ".to_owned(),
                                    range: None,
                                    role: BracketSourceFragmentRole::Normal,
                                });
                            }
                            children.push(fragment);
                        }
                        children.push(BracketSourceFragment::Text {
                            text: format!("{hair_space}{close}"),
                            range: *range,
                            role: BracketSourceFragmentRole::Normal,
                        });
                        vec![BracketSourceFragment::Span {
                            range: *range,
                            constructs: constructs.clone(),
                            children,
                        }]
                    }
                });
            }
        }
    }
    results
        .pop()
        .expect("fragment render keeps its root result")
}

#[invariant(true)]
#[invariant(::Enter { .. } => true)]
#[invariant(::Finish { .. } => true)]
enum FragmentJob<'a> {
    Enter {
        depth: usize,
        expr: &'a SExpr,
    },
    Finish {
        depth: usize,
        expr: &'a SExpr,
        child_count: usize,
    },
}

#[requires(true)]
#[ensures(true)]
fn source_fragment_is_empty(fragment: &BracketSourceFragment) -> bool {
    match fragment {
        BracketSourceFragment::Text { text, .. } => text.is_empty(),
        BracketSourceFragment::Span { children, .. } => children.is_empty(),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end))]
fn union_child_ranges(children: &[SExpr]) -> Option<BracketSourceRange> {
    let mut ranges = children.iter().filter_map(expr_range);
    let range = ranges.next()?;
    let mut byte_start = range.byte_start;
    let mut byte_end = range.byte_end;
    for child_range in ranges {
        byte_start = byte_start.min(child_range.byte_start);
        byte_end = byte_end.max(child_range.byte_end);
    }
    Some(new!(BracketSourceRange {
        byte_start,
        byte_end,
    }))
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end))]
pub(crate) fn expr_range(expr: &SExpr) -> Option<BracketSourceRange> {
    match expr {
        SExpr::Leaf { range, .. } | SExpr::Node { range, .. } => *range,
    }
}

#[requires(true)]
#[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
fn bracket_pair(depth: usize) -> (&'static str, &'static str) {
    match depth % 3 {
        0 => ("(", ")"),
        1 => ("[", "]"),
        _ => ("{", "}"),
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(!options.color && role != LeafRole::Error -> ret == old(text.clone()))]
#[ensures(!options.color && role == LeafRole::Error -> ret.starts_with('‼') && ret.ends_with('‼'))]
#[ensures(options.color && role != LeafRole::Error && !old(text.is_empty()) -> ret.starts_with(ansi_color_for_depth(depth)))]
#[ensures(options.color && role == LeafRole::Error -> ret.starts_with("\x1b[91m‼"))]
fn style_at_depth(
    depth: usize,
    text: String,
    options: BracketRenderOptions,
    role: LeafRole,
) -> String {
    render_bracketed_at_depth(
        depth,
        &SExpr::Leaf {
            text,
            range: None,
            role,
        },
        options,
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn ansi_color_for_depth(depth: usize) -> &'static str {
    match depth % 6 {
        0 => "\x1b[35m",
        1 => "\x1b[94m",
        2 => "\x1b[32m",
        3 => "\x1b[31m",
        4 => "\x1b[33m",
        _ => "\x1b[96m",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn ansi_parent_color_for_depth(depth: usize) -> &'static str {
    if depth == 0 {
        "\x1b[0m"
    } else {
        ansi_color_for_depth(depth - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn flatten_collapses_deeply_nested_single_child_groups() {
        let expr = node(vec![node(vec![node(vec![leaf(String::from("foo"))])])]);

        let flattened = flatten(expr);

        assert_eq!(flattened, leaf(String::from("foo")));
        assert_eq!(
            render_bracketed_with_options(&flattened, BracketRenderOptions::default()),
            "foo"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn render_collapses_single_non_empty_child_after_filtering_empty_children() {
        let expr = node(vec![empty_node(), leaf(String::from("foo")), empty_node()]);

        assert_eq!(
            render_bracketed_with_options(&expr, BracketRenderOptions::default()),
            "foo"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn color_cycle_uses_bright_blue_and_bright_cyan() {
        let expected = [
            "\x1b[35m", "\x1b[94m", "\x1b[32m", "\x1b[31m", "\x1b[33m", "\x1b[96m",
        ];

        for (depth, color) in expected.iter().enumerate() {
            assert_eq!(ansi_color_for_depth(depth), *color);
            assert_eq!(ansi_color_for_depth(depth + expected.len()), *color);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorized_text_restores_updated_parent_depth_color() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };

        assert_eq!(
            style_at_depth(0, String::from("foo"), options, LeafRole::Normal),
            "\x1b[35mfoo\x1b[0m"
        );
        assert_eq!(
            style_at_depth(2, String::from("foo"), options, LeafRole::Normal),
            "\x1b[32mfoo\x1b[94m"
        );
        assert_eq!(
            style_at_depth(6, String::from("foo"), options, LeafRole::Normal),
            "\x1b[35mfoo\x1b[96m"
        );
        assert_eq!(
            style_at_depth(1, String::from("foo"), options, LeafRole::Elided),
            "\x1b[94m\x1b[3mfoo\x1b[23m\x1b[35m"
        );
        assert_eq!(
            style_at_depth(2, String::from("ku"), options, LeafRole::Error),
            "\x1b[91m‼\x1b[9mku\x1b[29m‼\x1b[94m"
        );
        assert_eq!(
            style_at_depth(2, String::new(), options, LeafRole::Error),
            "\x1b[91m‼\x1b[9m\x1b[29m‼\x1b[94m"
        );
    }
}
