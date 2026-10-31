//! Music Theory shared kernel (`mc-theory`).
//!
//! Value objects and pure arithmetic shared by the Composition, Guitar and later
//! Analysis contexts. Kernel rule (ADR-019): only value objects and pure arithmetic;
//! searches, catalogues and algorithms belong to their owning context.
//!
//! Rule ids in comments and test names refer to docs/architecture/domain-invariants.md.

mod duration;
mod error;
pub mod fretted;
mod key;
mod pitch;
mod rational;
mod time;

pub use duration::{BaseValue, NoteValue, Tuplet};
pub use error::TheoryError;
pub use key::{KeySignature, Mode};
pub use pitch::{Alter, Letter, PitchClass, SpelledPitch};
pub use rational::Rational;
pub use time::{Tempo, TimeSignature};
