//! Stable ids of the pattern library. One counter per library allocates both kinds,
//! so a pattern id and a gesture id never coincide. Ids survive renames (ADR-034
//! section 6: a step refers to a gesture by id).

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

id_type!(PatternId);
id_type!(GestureId);
