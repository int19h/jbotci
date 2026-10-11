use jbotci_tree::{RecoveryItemKind, RecoveryItemState, tree_model};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecoveryTreeItem {
    Missing,
}

#[bityzba::contract_trait]
impl RecoveryItemState for RecoveryTreeItem {
    fn recovery_item_kind(&self) -> RecoveryItemKind {
        RecoveryItemKind::Missing
    }
}

tree_model! {
    #![tree_recovered]
    #![tree_view]

    #[derive(Debug)]
    pub enum Root { Original(std::string::String), Added(std::string::String) }
}

fn inspect(node: view::RootRef<'_>) {
    match node.get() { view::RootView::Original(_) => () }
}
fn main() {}
