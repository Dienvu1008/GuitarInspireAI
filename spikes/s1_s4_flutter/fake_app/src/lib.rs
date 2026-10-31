//! Stand-in for `mc-app`: application-layer types with no FFI knowledge.

/// Sum-type command with payloads, like the planned `mc-app::Command`.
#[derive(Clone, Debug)]
pub enum CommandKind {
    Noop,
    InsertNote { string_index: u8, fret: u8 },
    SetTempo { bpm_num: u32, bpm_den: u32 },
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub revision: u64,
    pub event_seq: u64,
}

#[derive(Default)]
pub struct Session {
    revision: u64,
    seq: u64,
    notes: Vec<(u8, u8)>,
}

impl Session {
    pub fn execute(&mut self, cmd: &CommandKind) -> Outcome {
        match cmd {
            CommandKind::Noop => {}
            CommandKind::InsertNote { string_index, fret } => self.notes.push((*string_index, *fret)),
            CommandKind::SetTempo { .. } => {}
        }
        self.revision += 1;
        self.seq += 1;
        Outcome { revision: self.revision, event_seq: self.seq }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}
