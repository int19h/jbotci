use super::*;

mod html;
mod markdown;

pub(crate) use html::render_block_html;
pub(crate) use markdown::render_block_markdown;

#[invariant(::KeepContent => true)]
#[invariant(::Drop => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CllPlainLinkDisposition {
    KeepContent,
    Drop,
}

impl CllLinkKind {
    /// Defines Plain behavior exhaustively for every semantic link kind.
    #[requires(true)]
    #[ensures(ret == CllPlainLinkDisposition::Drop -> self == CllLinkKind::Parse)]
    pub(crate) fn plain_disposition(self) -> CllPlainLinkDisposition {
        match self {
            // Section references carry a meaningful section title or number.
            CllLinkKind::Section => CllPlainLinkDisposition::KeepContent,
            // Example references carry a meaningful example label.
            CllLinkKind::Example => CllPlainLinkDisposition::KeepContent,
            // Dictionary links carry the referenced Lojban word.
            CllLinkKind::Dictionary => CllPlainLinkDisposition::KeepContent,
            // Rafsi links carry the referenced rafsi text.
            CllLinkKind::Rafsi => CllPlainLinkDisposition::KeepContent,
            // "Parse" is only an action; without its route it is noise.
            CllLinkKind::Parse => CllPlainLinkDisposition::Drop,
            // Asset links carry a descriptive label even without the path.
            CllLinkKind::Asset => CllPlainLinkDisposition::KeepContent,
            // External links carry descriptive text even without the URI.
            CllLinkKind::External => CllPlainLinkDisposition::KeepContent,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_href(site: &CllSite, href: &str) -> String {
    if let Some(target) = href.strip_prefix("../vlacku/") {
        return cll_link_href(site, CllLinkKind::Dictionary, target);
    }
    href.to_owned()
}
