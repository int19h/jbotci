use bityzba::{invariant, new};

#[invariant(!name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Name {
    name: String,
}

fn require_serde<T>()
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
}

fn main() {
    require_serde::<Name>();

    let name = new!(Name {
        name: String::from("cmavo"),
    });
    let json = serde_json::to_string(&name).expect("serialize wrapper");
    let parsed: Name = serde_json::from_str(&json).expect("deserialize wrapper");
    assert_eq!(parsed, name);
}
