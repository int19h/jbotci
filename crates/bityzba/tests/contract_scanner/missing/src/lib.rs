struct MissingType {
    value: usize,
}

enum MissingEnum {
    Empty,
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
