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
    pub struct Root {
        pub text: std::string::String,
        pub added: std::string::String,
    }
}

fn inspect(node: view::RootRef<'_>) {
    let _ = |view::RootView { text: _ }: view::RootView<'_>| ();
    let _ = node.field_text();
}
fn main() {}
