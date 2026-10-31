//! Stable entity ids. One counter per score allocates all kinds, so ids are unique
//! across tracks, beats and notes (H-01). Ids survive edits and saves.

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            pub const fn from_raw(raw: u64) -> Self {
                $name(raw)
            }

            pub const fn raw(self) -> u64 {
                self.0
            }
        }
    };
}

id_type!(TrackId);
id_type!(BeatId);
id_type!(NoteId);
