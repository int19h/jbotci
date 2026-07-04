use super::*;

mod html;
mod markdown;

pub(crate) use html::render_block_html;
pub(crate) use markdown::render_block_markdown;

#[requires(true)]
#[ensures(true)]
fn render_ebnf_href(site: &CllSite, href: &str) -> String {
    if let Some(target) = href.strip_prefix("../vlacku/") {
        return cll_link_href(site, CllLinkKind::Dictionary, target);
    }
    href.to_owned()
}
