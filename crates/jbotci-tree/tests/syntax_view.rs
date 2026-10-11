//! The generated view preserves shape and borrows without heap allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use bityzba::{invariant, requires};
use jbotci_tree::{RecoveryItemKind, RecoveryItemState, tree_model};

thread_local! { static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) }; }

#[invariant(true)]
struct CountingAllocator;

// Only the current test thread counts allocations inside the measured operation.
unsafe impl GlobalAlloc for CountingAllocator {
    #[requires(true)]
    #[ensures(true)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    #[requires(true)]
    #[ensures(true)]
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[invariant(true)]
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
    #[invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Root { pub children: Vec<Arc<Choice>>, pub optional: Option<Arc<Child>> }
    #[invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    #[invariant(::Child(_) => true)]
    #[invariant(::Named => true)]
    pub enum Choice { Child(Arc<Child>), Named { child: Arc<Child> } }
    #[invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Child { pub number: std::string::String }
}

#[invariant(true)]
struct Counter {
    atoms: usize,
    errors: usize,
}

#[bityzba::contract_trait]
impl<'tree> view::Walker<'tree> for Counter {
    #[requires(true)]
    #[ensures(self.atoms == old(self.atoms) + 1)]
    fn walk_atom(&mut self, _atom: AtomRef<'tree>) {
        self.atoms += 1;
    }
    #[requires(true)]
    #[ensures(self.errors == old(self.errors) + 1)]
    fn walk_error(&mut self) {
        self.errors += 1;
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn projections_and_walks_borrow_without_allocation() {
    use view::{AsView, Walkable};
    let strict = Root {
        children: vec![Arc::new(Choice::Child(Arc::new(Child {
            number: String::from("seven"),
        })))],
        optional: None,
    };
    let mut recovered: recovered::Root = recovered::Root::from_valid(strict.clone());
    recovered.optional = Some(Arc::new(recovered::Recovered::Error(
        RecoveryTreeItem::Missing,
    )));
    recovered
        .children
        .push(Arc::new(recovered::Recovered::Prefix(
            jbotci_tree::RecoveredPrefix {
                errors: vec1::vec1![RecoveryTreeItem::Missing],
                value: Box::new(recovered::Choice::Named {
                    child: Arc::new(recovered::Recovered::valid(recovered::Child {
                        number: recovered::Recovered::valid(String::from("nine")),
                    })),
                }),
            },
        )));
    let recovered = recovered::Recovered::valid(recovered);
    ALLOCATIONS.with(|count| count.set(Some(0)));
    assert!(!strict.as_view().valid().unwrap().has_errors());
    assert!(recovered.as_view().valid().unwrap().has_errors());
    let view::RootView { children, optional } = strict.as_view().valid().unwrap().get();
    assert!(optional.is_none());
    assert_eq!(children.len(), 1);
    let view::ChoiceView::Child(child) = children.get(0).unwrap().valid().unwrap().get() else {
        panic!("strict arm changed")
    };
    let view::ChildView { number } = child.valid().unwrap().get();
    assert_eq!(number.valid().map(String::as_str), Some("seven"));
    let view::RootView { children, optional } = recovered.as_view().valid().unwrap().get();
    assert!(optional.unwrap().valid().is_none());
    assert_eq!(children.len(), 2);
    assert!(children.get(1).unwrap().valid().is_none());
    let mut counter = Counter {
        atoms: 0,
        errors: 0,
    };
    recovered.as_view().walk_with(&mut counter);
    assert_eq!(counter.atoms, 2);
    assert_eq!(counter.errors, 2);
    let mut value_counter = Counter {
        atoms: 0,
        errors: 0,
    };
    assert!(view::walk::value(
        &mut value_counter,
        children.get(1).unwrap()
    ));
    assert_eq!(value_counter.atoms, 1);
    assert_eq!(value_counter.errors, 0);
    let allocations = ALLOCATIONS.with(|count| count.replace(None)).unwrap();
    assert_eq!(allocations, 0);
}
