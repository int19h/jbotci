def smoke() -> str:
    """Confirm that the native extension loaded and can execute Rust code."""

def raise_sample_error(message: str) -> None:
    """Raise a sample structured error through the shared conversion path."""

@final
class SampleMode(StrEnum):
    """Temporary fieldless enum used to test stable string registration."""

    BASIC = "basic"
    ADVANCED = "advanced"

def sample_mode(advanced: bool = False) -> SampleMode:
    """Return a sample enum through the stable string conversion path."""

@final
class Sample:
    """Temporary immutable value object used to test binding conventions."""

    def __new__(cls, value: str) -> Sample: ...
    @property
    def value(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
