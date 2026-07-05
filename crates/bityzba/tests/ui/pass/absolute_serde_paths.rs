mod serde {
    pub trait Serialize {}
    pub trait Deserialize<'de> {}
}

#[bityzba::invariant(*value > 0)]
#[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
struct ShadowedSerde {
    value: usize,
}

fn main() {
    let value = ShadowedSerde::from_data(ShadowedSerdeData { value: 1 });
    let _ = value;
}
