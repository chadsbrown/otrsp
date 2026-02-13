/// Which radio (1 or 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radio {
    Radio1,
    Radio2,
}

/// Receive audio routing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxMode {
    /// Selected radio audio in both ears.
    Mono,
    /// Radio 1 left ear, Radio 2 right ear.
    Stereo,
    /// Radio 1 right ear, Radio 2 left ear.
    ReverseStereo,
}
