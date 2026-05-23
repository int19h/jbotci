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

impl MissingType {
    fn update(&mut self) {
        self.value += 1;
    }
}
