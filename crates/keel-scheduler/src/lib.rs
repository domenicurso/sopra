mod blink;
mod clock;

pub use blink::CursorBlink;
pub use clock::{FrameClock, FrameDecision, InvalidationReason};

#[cfg(test)]
mod tests;
