#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, requires};

use super::*;

#[contract_trait]
pub(crate) trait CllBlockVisitor {
    #[requires(true)]
    #[ensures(true)]
    fn visit_blocks(&mut self, blocks: &[CllBlock]) {
        walk_blocks(self, blocks);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_block(&mut self, block: &CllBlock) {
        walk_block(self, block);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline_run(&mut self, inlines: &[CllInline]) {
        walk_inline_run(self, inlines);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline(&mut self, inline: &CllInline) {
        walk_inline(self, inline);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_blocks_mut(&mut self, blocks: &mut [CllBlock]) {
        walk_blocks_mut(self, blocks);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_block_mut(&mut self, block: &mut CllBlock) {
        walk_block_mut(self, block);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline_run_mut(&mut self, inlines: &mut [CllInline]) {
        walk_inline_run_mut(self, inlines);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline_mut(&mut self, inline: &mut CllInline) {
        walk_inline_mut(self, inline);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_blocks<V: CllBlockVisitor + ?Sized>(visitor: &mut V, blocks: &[CllBlock]) {
    for block in blocks {
        visitor.visit_block(block);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_block<V: CllBlockVisitor + ?Sized>(visitor: &mut V, block: &CllBlock) {
    match block {
        CllBlock::Paragraph { inlines, .. } => visitor.visit_inline_run(inlines),
        CllBlock::List { items, .. } => {
            for item in items {
                visitor.visit_blocks(item);
            }
        }
        CllBlock::Example { .. } => {}
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            if let Some(caption) = caption {
                visitor.visit_inline_run(caption);
            }
            for row in header_rows.iter().chain(body_rows.iter()) {
                for cell in row {
                    visitor.visit_blocks(&cell.blocks);
                }
            }
        }
        CllBlock::SimpleListTable { rows, .. } => {
            for cell in rows.iter().flatten().flatten() {
                visitor.visit_inline_run(cell);
            }
        }
        CllBlock::VariableList { entries, .. } => {
            for entry in entries {
                visitor.visit_inline_run(&entry.term);
                visitor.visit_blocks(&entry.blocks);
            }
        }
        CllBlock::Media { title, .. } => {
            if let Some(title) = title {
                visitor.visit_inline_run(title);
            }
        }
        CllBlock::Rule { body, .. } => visitor.visit_blocks(body),
        CllBlock::Code { .. } => {}
        CllBlock::Heading { inlines, .. } => visitor.visit_inline_run(inlines),
        CllBlock::BlockQuote { blocks, .. } => visitor.visit_blocks(blocks),
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            visitor.visit_inline_run(body);
        }
        CllBlock::InterlinearGloss {
            rows,
            natlang,
            comments,
            ..
        } => {
            for row in rows {
                for cell in &row.cells {
                    visitor.visit_inline_run(cell);
                }
            }
            for line in natlang.iter().chain(comments.iter()) {
                visitor.visit_inline_run(line);
            }
        }
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => {
            for line in titles.iter().chain(headers.iter()) {
                visitor.visit_inline_run(line);
            }
            for cell in rows.iter().flatten() {
                visitor.visit_inline_run(cell);
            }
        }
        CllBlock::Lojbanization { lines, .. } => {
            for line in lines {
                visitor.visit_inline_run(&line.body);
                if let Some(comment) = &line.comment {
                    visitor.visit_inline_run(comment);
                }
            }
        }
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                visitor.visit_inline_run(&part.body);
            }
        }
        CllBlock::Ebnf { .. } | CllBlock::DisplayMath { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_inline_run<V: CllBlockVisitor + ?Sized>(visitor: &mut V, inlines: &[CllInline]) {
    for inline in inlines {
        visitor.visit_inline(inline);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_inline<V: CllBlockVisitor + ?Sized>(visitor: &mut V, inline: &CllInline) {
    match inline {
        CllInline::Emphasis { inlines, .. }
        | CllInline::Quote { inlines, .. }
        | CllInline::LanguageSpan { inlines, .. }
        | CllInline::CiteTitle { inlines }
        | CllInline::Subscript { inlines }
        | CllInline::Superscript { inlines }
        | CllInline::Link { inlines, .. }
        | CllInline::Elidable { inlines, .. } => visitor.visit_inline_run(inlines),
        CllInline::Text(_)
        | CllInline::Code(_)
        | CllInline::InlineMath { .. }
        | CllInline::Anchor { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_blocks_mut<V: CllBlockVisitor + ?Sized>(
    visitor: &mut V,
    blocks: &mut [CllBlock],
) {
    for block in blocks {
        visitor.visit_block_mut(block);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_block_mut<V: CllBlockVisitor + ?Sized>(visitor: &mut V, block: &mut CllBlock) {
    match block {
        CllBlock::Paragraph { inlines, .. } => visitor.visit_inline_run_mut(inlines),
        CllBlock::List { items, .. } => {
            for item in items {
                visitor.visit_blocks_mut(item);
            }
        }
        CllBlock::Example { .. } => {}
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            if let Some(caption) = caption {
                visitor.visit_inline_run_mut(caption);
            }
            for row in header_rows.iter_mut().chain(body_rows.iter_mut()) {
                *row = std::mem::take(row)
                    .into_iter()
                    .map(|cell| {
                        let data = cell.into_data();
                        let mut blocks = data.blocks;
                        visitor.visit_blocks_mut(&mut blocks);
                        CllTableCell::from_data(data!(CllTableCell { blocks, ..data }))
                    })
                    .collect();
            }
        }
        CllBlock::SimpleListTable { rows, .. } => {
            for cell in rows.iter_mut().flatten().flatten() {
                visitor.visit_inline_run_mut(cell);
            }
        }
        CllBlock::VariableList { entries, .. } => {
            *entries = std::mem::take(entries)
                .into_iter()
                .map(|entry| {
                    let data = entry.into_data();
                    let mut term = data.term;
                    let mut blocks = data.blocks;
                    visitor.visit_inline_run_mut(&mut term);
                    visitor.visit_blocks_mut(&mut blocks);
                    CllVariableEntry::from_data(data!(CllVariableEntry { term, blocks }))
                })
                .collect();
        }
        CllBlock::Media { title, .. } => {
            if let Some(title) = title {
                visitor.visit_inline_run_mut(title);
            }
        }
        CllBlock::Rule { body, .. } => visitor.visit_blocks_mut(body),
        CllBlock::Code { .. } => {}
        CllBlock::Heading { inlines, .. } => visitor.visit_inline_run_mut(inlines),
        CllBlock::BlockQuote { blocks, .. } => visitor.visit_blocks_mut(blocks),
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            visitor.visit_inline_run_mut(body);
        }
        CllBlock::InterlinearGloss {
            rows,
            natlang,
            comments,
            ..
        } => {
            *rows = std::mem::take(rows)
                .into_iter()
                .map(|row| {
                    let data = row.into_data();
                    let mut cells = data.cells;
                    for cell in &mut cells {
                        visitor.visit_inline_run_mut(cell);
                    }
                    CllInterlinearRow::from_data(data!(CllInterlinearRow {
                        kind: data.kind,
                        cells,
                    }))
                })
                .collect();
            for line in natlang.iter_mut().chain(comments.iter_mut()) {
                visitor.visit_inline_run_mut(line);
            }
        }
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => {
            for title in titles.iter_mut().chain(headers.iter_mut()) {
                visitor.visit_inline_run_mut(title);
            }
            for cell in rows.iter_mut().flatten() {
                visitor.visit_inline_run_mut(cell);
            }
        }
        CllBlock::Lojbanization { lines, .. } => {
            *lines = std::mem::take(lines)
                .into_iter()
                .map(|line| {
                    let data = line.into_data();
                    let mut body = data.body;
                    let mut comment = data.comment;
                    visitor.visit_inline_run_mut(&mut body);
                    if let Some(comment) = &mut comment {
                        visitor.visit_inline_run_mut(comment);
                    }
                    CllLojbanizationLine::from_data(data!(CllLojbanizationLine {
                        kind: data.kind,
                        body,
                        comment,
                    }))
                })
                .collect();
        }
        CllBlock::LujvoMaking { parts, .. } => {
            *parts = std::mem::take(parts)
                .into_iter()
                .map(|part| {
                    let data = part.into_data();
                    let mut body = data.body;
                    visitor.visit_inline_run_mut(&mut body);
                    CllLujvoPart::from_data(data!(CllLujvoPart {
                        kind: data.kind,
                        body,
                    }))
                })
                .collect();
        }
        CllBlock::Ebnf { .. } | CllBlock::DisplayMath { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_inline_run_mut<V: CllBlockVisitor + ?Sized>(
    visitor: &mut V,
    inlines: &mut [CllInline],
) {
    for inline in inlines {
        visitor.visit_inline_mut(inline);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn walk_inline_mut<V: CllBlockVisitor + ?Sized>(
    visitor: &mut V,
    inline: &mut CllInline,
) {
    match inline {
        CllInline::Emphasis { inlines, .. }
        | CllInline::Quote { inlines, .. }
        | CllInline::LanguageSpan { inlines, .. }
        | CllInline::CiteTitle { inlines }
        | CllInline::Subscript { inlines }
        | CllInline::Superscript { inlines }
        | CllInline::Link { inlines, .. }
        | CllInline::Elidable { inlines, .. } => visitor.visit_inline_run_mut(inlines),
        CllInline::Text(_)
        | CllInline::Code(_)
        | CllInline::InlineMath { .. }
        | CllInline::Anchor { .. } => {}
    }
}
