use jbotci_tree::{RecoveryItemKind, RecoveryItemState, tree_model};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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

    #[derive(Debug)]
    pub struct Root {
        pub text: std::string::String,
    }
}

fn main() {
    let root = Root {
        text: String::from("text"),
    };
    let _ = root;
}
