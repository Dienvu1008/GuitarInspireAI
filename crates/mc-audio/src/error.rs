//! Errors of the audio engine. Messages are for the user and for logs; they never
//! contain file contents.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioError {
    /// The system has no default output device (playback is disabled, editing works).
    NoOutputDevice,
    /// The output device does not offer 32-bit float samples.
    UnsupportedFormat(String),
    /// The audio stream could not be built or started.
    Stream(String),
    /// The SoundFont is missing, unreadable or invalid (playback is disabled).
    SoundFont(String),
    /// The engine thread is gone.
    EngineStopped,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoOutputDevice => write!(f, "no audio output device"),
            AudioError::UnsupportedFormat(s) => write!(f, "unsupported audio format: {s}"),
            AudioError::Stream(s) => write!(f, "audio stream error: {s}"),
            AudioError::SoundFont(s) => write!(f, "SoundFont error: {s}"),
            AudioError::EngineStopped => write!(f, "the audio engine has stopped"),
        }
    }
}

impl std::error::Error for AudioError {}
