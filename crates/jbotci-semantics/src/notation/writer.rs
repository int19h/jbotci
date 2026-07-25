//! Physical writer for the model-facing notation (Proposal v5, `smusni` profile).
//!
//! This is a faithful Rust port of the Python oracle's `Writer` class plus its
//! two dense-declaration helpers (`experiments/notation-renderer-v0/render_v5.py`
//! at commit `cab176bcce`, the byte-parity oracle fixed by the research repo's
//! `FREEZE-PHASE-B.md`). It enforces the five disjoint punctuation roles of the
//! notation as distinct call shapes so a renderer cannot accidentally blend them:
//!
//! * [`Writer::field`] — `LABEL: value;` scalar edges (role 1);
//! * [`Writer::heading`] — a structured field introduced by a bare label (role 2);
//! * [`Writer::ordered`] — `LABEL ( ... )` order-bearing sequences (role 3);
//! * [`Writer::collection`] — `LABEL { ... }` collections/maps/sets (role 5);
//! * [`Writer::entry`] — one `text;` element inside an ordered/collection body (role 6);
//! * [`Writer::declaration`] — `<KIND> <identity> ...` identified declarations (§3.1),
//!   with the `smusni` brace closer (`opt_braces`, N4) and one-line collapsing
//!   (`opt_dense_decls`, N8e).
//!
//! The dense-declaration machinery captures a declaration's body into a scratch
//! frame (via `collect_stack`), counts its direct fields with
//! [`count_direct_units`], and — at or under four — collapses the captured lines
//! onto one line with [`flatten_body`]. Over four, it replays the captured lines
//! verbatim, byte-identical to the option being off. The comments on
//! [`Writer::declaration`], [`count_direct_units`], and [`flatten_body`] restate
//! the frozen rules verbatim from `FREEZE-PHASE-B.md` section (c).

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

/// The physical line writer. Roles are separate methods so their punctuation
/// cannot be blended; `opt_braces`/`opt_dense_decls` are the two `smusni` shape
/// options that alter [`Writer::declaration`] only.
///
/// `collect_stack` is empty except while a dense declaration's body is being
/// captured; [`Writer::emit`] routes to the innermost open frame when one
/// exists, so every nested role call is transparently redirected without any
/// role method needing to know capture is happening — exactly the property that
/// guarantees the "not dense enough" fallback is byte-identical to the option
/// being off.
// `#[invariant(true)]`: `Writer` is a mutable line-accumulating builder, not a
// validated model type, so it takes the audited no-op marker (which does not
// generate a bityzba data wrapper). Its structural discipline — balanced
// indent, capture-frame nesting — is enforced by the RAII-style `body` closures
// of `heading`/`ordered`/`collection`/`declaration`, not by a field invariant.
#[invariant(true)]
pub struct Writer {
    lines: Vec<String>,
    indent: usize,
    opt_braces: bool,
    opt_dense_decls: bool,
    collect_stack: Vec<Vec<String>>,
}

/// The notation indents two spaces per nesting level.
const INDENT_WIDTH: usize = 2;

impl Writer {
    #[requires(true)]
    #[ensures(ret.lines.is_empty() && ret.indent == 0 && ret.collect_stack.is_empty())]
    #[ensures(ret.opt_braces == opt_braces && ret.opt_dense_decls == opt_dense_decls)]
    pub fn new(opt_braces: bool, opt_dense_decls: bool) -> Self {
        Self {
            lines: Vec::new(),
            indent: 0,
            opt_braces,
            opt_dense_decls,
            collect_stack: Vec::new(),
        }
    }

    /// The accumulated document text, one trailing newline, matching the
    /// oracle's `"\n".join(lines) + "\n"`.
    #[requires(true)]
    #[ensures(ret.ends_with('\n'))]
    pub fn finish(self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }

    /// Emit one already-labelled line at the current indent, routed to the
    /// innermost open capture frame when one exists, else to the document.
    #[requires(true)]
    #[ensures(true)]
    fn emit(&mut self, text: &str) {
        let line = format!("{}{}", " ".repeat(INDENT_WIDTH * self.indent), text);
        match self.collect_stack.last_mut() {
            Some(frame) => frame.push(line),
            None => self.lines.push(line),
        }
    }

    /// Role 1: `LABEL: value;` — a scalar or single-edge association.
    #[requires(true)]
    #[ensures(true)]
    pub fn field(&mut self, label: &str, value: &str) {
        self.emit(&format!("{label}: {value};"));
    }

    /// Role 6: a semicolon-terminated entry inside a collection or ordered
    /// sequence.
    #[requires(true)]
    #[ensures(true)]
    pub fn entry(&mut self, text: &str) {
        self.emit(&format!("{text};"));
    }

    /// A compact `LABEL value` line with no colon and no trailing semicolon —
    /// used where a value is a derived/summary fact rather than an ordinary
    /// scalar edge (the tested-winner `DENOTES VALUES OF SORT <Sort>` header).
    #[requires(true)]
    #[ensures(true)]
    pub fn annotate(&mut self, label: &str, value: &str) {
        self.emit(&format!("{label} {value}"));
    }

    /// Role 2: a structured field introduced by a heading — no colon, no
    /// field-level semicolon on the body. `body` renders the indented children.
    #[requires(true)]
    #[ensures(true)]
    pub fn heading(&mut self, label: &str, body: impl FnOnce(&mut Self)) {
        self.emit(label);
        self.indent += 1;
        body(self);
        self.indent -= 1;
    }

    /// Role 3: parentheses around an ordered sequence.
    #[requires(true)]
    #[ensures(true)]
    pub fn ordered(&mut self, label: &str, body: impl FnOnce(&mut Self)) {
        self.emit(&format!("{label} ("));
        self.indent += 1;
        body(self);
        self.indent -= 1;
        self.emit(")");
    }

    /// Role 5: braces around a collection/map/set.
    #[requires(true)]
    #[ensures(true)]
    pub fn collection(&mut self, label: &str, body: impl FnOnce(&mut Self)) {
        self.emit(&format!("{label} {{"));
        self.indent += 1;
        body(self);
        self.indent -= 1;
        self.emit("}");
    }

    /// The eventuality dimension record as one compact, COMPLETE named-tuple
    /// line (all eight dimensions, per the explicit-over-implicit ruling)
    /// instead of eight `LABEL: value;` lines. Wraps onto multiple lines only
    /// when the single-line form would exceed `max_width` columns at the
    /// current indent (the oracle's `Writer.dimension_record`, `max_width=100`).
    #[requires(true)]
    #[ensures(true)]
    pub fn dimension_record(&mut self, ref_id: &str, pairs: &[(String, String)]) {
        const MAX_WIDTH: usize = 100;
        let inline = pairs
            .iter()
            .map(|(k, v)| format!("{k} = {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        let oneline = format!("{ref_id}.{{ {inline} }}");
        if INDENT_WIDTH * self.indent + oneline.chars().count() <= MAX_WIDTH {
            self.emit(&oneline);
            return;
        }
        self.emit(&format!("{ref_id}.{{"));
        self.indent += 1;
        for (index, (k, v)) in pairs.iter().enumerate() {
            let comma = if index < pairs.len() - 1 { "," } else { "" };
            self.emit(&format!("{k} = {v}{comma}"));
        }
        self.indent -= 1;
        self.emit("}");
    }

    /// §3.1: `<KIND> <identity> [IS <variant>] ... ` declaration, with the
    /// `smusni` brace closer and one-line collapsing.
    ///
    /// N4 (`opt_braces`): the opener line gains a trailing ` {` and the closer
    /// drops the `END <KIND> <identity>;` repetition in favour of a bare `}`.
    ///
    /// N8e (`opt_dense_decls`): when `opt_dense_decls` is set AND
    /// `dense_eligible` (the top-level `SEMANTIC DOCUMENT` wrapper passes
    /// `dense_eligible = false` — its own trio would otherwise itself look
    /// "dense"), the body is rendered into a scratch capture frame using the
    /// SAME role calls as always, then [`count_direct_units`] counts the body's
    /// own direct fields. At or under four, [`flatten_body`] collapses the
    /// captured lines onto one `<KIND> <identity> { ... }` line. Over four, the
    /// captured lines are replayed verbatim — byte-for-byte what non-dense
    /// rendering already produces.
    #[requires(variant.is_none_or(|variant| !variant.is_empty()))]
    #[ensures(true)]
    pub fn declaration(
        &mut self,
        kind: &str,
        identity: &str,
        variant: Option<&str>,
        dense_eligible: bool,
        body: impl FnOnce(&mut Self),
    ) {
        let head = match variant {
            Some(variant) => format!("{kind} {identity} IS {variant}"),
            None => format!("{kind} {identity}"),
        };
        if self.opt_dense_decls && dense_eligible {
            let base_indent = self.indent;
            self.collect_stack.push(Vec::new());
            self.indent += 1;
            body(self);
            self.indent -= 1;
            let body_lines = self
                .collect_stack
                .pop()
                .expect("a capture frame was pushed for this declaration");
            let child_indent = base_indent + 1;
            if count_direct_units(&body_lines, child_indent, INDENT_WIDTH) <= 4 {
                let flat = flatten_body(&body_lines, child_indent, INDENT_WIDTH);
                let text = if flat.is_empty() {
                    format!("{head} {{ }}")
                } else {
                    format!("{head} {{ {flat} }}")
                };
                self.emit_at(base_indent, &text);
            } else {
                let opener = if self.opt_braces {
                    format!("{head} {{")
                } else {
                    head.clone()
                };
                self.emit_at(base_indent, &opener);
                // `collect_stack` is empty here (declarations are never nested
                // in this profile — each object is a sibling under the single
                // DECLARATIONS collection, which is not a capture frame), so the
                // replayed body lands directly in the document, byte-identical
                // to the option being off.
                match self.collect_stack.last_mut() {
                    Some(frame) => frame.extend(body_lines),
                    None => self.lines.extend(body_lines),
                }
                let closer = if self.opt_braces {
                    "}".to_string()
                } else {
                    format!("END {kind} {identity};")
                };
                self.emit_at(base_indent, &closer);
            }
            return;
        }
        let opener = if self.opt_braces {
            format!("{head} {{")
        } else {
            head.clone()
        };
        self.emit(&opener);
        self.indent += 1;
        body(self);
        self.indent -= 1;
        if self.opt_braces {
            self.emit("}");
        } else {
            self.emit(&format!("END {kind} {identity};"));
        }
    }

    /// Emit `text` at an explicit indent level, restoring the previous indent.
    /// Used by the dense-declaration branch, which emits the head/closer at the
    /// declaration's own (shallower) indent than its captured body.
    #[requires(true)]
    #[ensures(true)]
    fn emit_at(&mut self, indent: usize, text: &str) {
        let prev = self.indent;
        self.indent = indent;
        self.emit(text);
        self.indent = prev;
    }
}

/// The indent depth (in levels) and stripped text of one captured line, per the
/// oracle's `_line_depth_and_text` (leading spaces divided by the indent width).
#[requires(indent_width > 0)]
#[ensures(true)]
fn line_depth_and_text(line: &str, indent_width: usize) -> (usize, &str) {
    let stripped = line.trim_start_matches(' ');
    let depth = (line.len() - stripped.len()) / indent_width;
    (depth, stripped)
}

/// Whether a captured line is a self-delimiting group opener, matched against
/// the EXACT three opener suffixes the [`Writer`] emits — `ordered`'s ` (`,
/// `collection`'s (and a synthesized `heading`'s) ` {`, and `dimension_record`'s
/// `.{` — rather than a bare trailing `{`/`(`. Every non-opener leaf line is
/// either `;`-terminated (`field`/`entry`), ends in a closing quote `"` (all
/// free/witness text goes through [`super::render`]'s `quote`), or is a closed
/// sort/enum word (`annotate`, `dimension_record` pairs) — none of which can end
/// in ` (`, ` {`, or `.{`. So this never false-positives on a field value that
/// merely happens to contain a brace/paren (adversarial review round 1;
/// lockstep with the oracle's `_is_group_opener`).
#[requires(true)]
#[ensures(ret == (text.ends_with(" (") || text.ends_with(" {") || text.ends_with(".{")))]
fn is_group_opener(text: &str) -> bool {
    text.ends_with(" (") || text.ends_with(" {") || text.ends_with(".{")
}

/// How many direct fields a declaration's captured body has, at `child_indent`
/// (one level deeper than the declaration's own head line): one whole
/// self-delimiting group (`ARGS (...)`, `DESCRIPTOR IS ... {...}`) or heading
/// group (`SCOPE DEPENDENCE ...` plus its indented children) counts as exactly
/// one unit — matching what a human reading "how many fields does this
/// declaration have" would count, not one unit per physical line. Verbatim port
/// of the oracle's `_count_direct_units`.
#[requires(indent_width > 0)]
#[ensures(true)]
pub fn count_direct_units(body_lines: &[String], child_indent: usize, indent_width: usize) -> usize {
    let mut count = 0;
    let mut i = 0;
    let n = body_lines.len();
    while i < n {
        let (depth, text) = line_depth_and_text(&body_lines[i], indent_width);
        if depth != child_indent {
            i += 1;
            continue;
        }
        count += 1;
        if is_group_opener(text) {
            let closer = if text.ends_with('{') { '}' } else { ')' };
            let mut depth_count = 1;
            i += 1;
            while i < n && depth_count > 0 {
                let (depth2, text2) = line_depth_and_text(&body_lines[i], indent_width);
                if depth2 == child_indent && text2.len() == 1 && text2.starts_with(closer) {
                    depth_count -= 1;
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    count
}

/// Collapse a captured declaration body onto one space-joined line,
/// reconstructing nesting from indentation alone. Self-delimiting groups
/// (`ordered`/`collection`, and a wrapped `dimension_record`) keep exactly the
/// bracket text they already have — only their inner leaf lines get
/// semicolon-terminated if they were not already. A bare-label `heading()`
/// group — no closer at all in normal multi-line output — gets a synthetic
/// `{...}` added so the flattened form stays unambiguous to read. Verbatim port
/// of the oracle's `_flatten_body`.
#[requires(indent_width > 0)]
#[ensures(true)]
pub fn flatten_body(body_lines: &[String], child_indent: usize, indent_width: usize) -> String {
    let (flat, _) = flatten_span(body_lines, 0, child_indent, indent_width);
    flat
}

/// One recursive span of [`flatten_body`], returning the joined text and the
/// index one past the span. Mirrors the oracle's inner `render_span` closure.
#[requires(indent_width > 0)]
#[ensures(ret.1 >= start)]
fn flatten_span(
    body_lines: &[String],
    start: usize,
    depth: usize,
    indent_width: usize,
) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    let n = body_lines.len();
    let mut i = start;
    while i < n {
        let (line_depth, text) = line_depth_and_text(&body_lines[i], indent_width);
        if line_depth < depth {
            break;
        }
        if is_group_opener(text) {
            let closer = if text.ends_with('{') { '}' } else { ')' };
            parts.push(text.to_string());
            let (inner, next) = flatten_span(body_lines, i + 1, depth + 1, indent_width);
            parts.push(inner);
            i = next;
            if i < n {
                let (_, text_at_i) = line_depth_and_text(&body_lines[i], indent_width);
                if text_at_i.len() == 1 && text_at_i.starts_with(closer) {
                    parts.push(closer.to_string());
                    i += 1;
                }
            }
            continue;
        }
        if i + 1 < n && line_depth_and_text(&body_lines[i + 1], indent_width).0 > line_depth {
            parts.push(format!("{text} {{"));
            let (inner, next) = flatten_span(body_lines, i + 1, depth + 1, indent_width);
            parts.push(inner);
            parts.push("}".to_string());
            i = next;
            continue;
        }
        let mut frag = text.trim_end_matches(',').to_string();
        if !frag.ends_with(';') {
            frag.push(';');
        }
        parts.push(frag);
        i += 1;
    }
    (parts.join(" "), i)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blocker-3 regression (round-1 review, kimi 5): a leaf whose value merely
    /// ends in a bare `{`/`(` (a hypothetical future unquoted field, or a
    /// hostile witness value), and a quoted value carrying `{ ( ; } )`, must not
    /// be mistaken for a group opener. `is_group_opener` matches only the exact
    /// ` (`/` {`/`.{` suffixes the Writer emits, so both stay leaves.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn flatten_does_not_treat_hostile_leaf_values_as_group_openers() {
        // A captured declaration body at child_indent = 1 (two-space indent):
        // a real ordered group, a quoted hostile value, and a hypothetical
        // unquoted value ending in a bare `(`.
        let body = vec![
            "  ARGS (".to_string(),
            "    [1]: r1;".to_string(),
            "  )".to_string(),
            "  NAME: \"a{b(c;d}e\";".to_string(),
            "  WEIRD value(".to_string(),
        ];
        // Three direct units: the ARGS group counts as one (not consuming the
        // hostile lines after it); the two leaves are one each.
        assert_eq!(count_direct_units(&body, 1, 2), 3);
        assert_eq!(
            flatten_body(&body, 1, 2),
            "ARGS ( [1]: r1; ) NAME: \"a{b(c;d}e\"; WEIRD value(;"
        );
    }

    /// A heading group nested inside a collection (the `ASSIGNED NAMES { ASSIGNED
    /// NAME … }` shape introduced this round) flattens with the heading's
    /// synthetic braces and the collection's own braces intact — the whole
    /// collection is one direct unit.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn flatten_handles_heading_group_inside_collection() {
        let body = vec![
            "  ASSIGNED NAMES {".to_string(),
            "    ASSIGNED NAME".to_string(),
            "      WORD: ko'a;".to_string(),
            "      NAME: \"ko'a\";".to_string(),
            "      INTRODUCED BY: goi;".to_string(),
            "  }".to_string(),
        ];
        assert_eq!(count_direct_units(&body, 1, 2), 1);
        assert_eq!(
            flatten_body(&body, 1, 2),
            "ASSIGNED NAMES { ASSIGNED NAME { WORD: ko'a; NAME: \"ko'a\"; INTRODUCED BY: goi; } }"
        );
    }

    /// `is_group_opener` accepts exactly the three suffixes the Writer emits and
    /// rejects bare-brace/paren leaf endings.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn group_opener_matches_only_writer_suffixes() {
        assert!(is_group_opener("ARGS ("));
        assert!(is_group_opener("ASSIGNED NAMES {"));
        assert!(is_group_opener("r6.{"));
        assert!(!is_group_opener("WEIRD value("));
        assert!(!is_group_opener("NAME: \"x{\";"));
        assert!(!is_group_opener("DENOTES VALUES OF SORT Entity"));
    }
}
