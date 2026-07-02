struct MissingType {
    value: usize,
}

enum MissingEnum {
    Empty,
}

#[invariant(true)]
enum MissingVariantInvariant {
    Empty,
    Present { value: usize },
}

trait MissingTrait {
    fn parse_term(&self);
}

fn parse_term() {}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| *value > 0))]
fn unsound_result_contract() -> Result<usize, String> {
    Ok(1)
}

impl MissingType {
    fn update(&mut self) {
        self.value += 1;
    }
}
