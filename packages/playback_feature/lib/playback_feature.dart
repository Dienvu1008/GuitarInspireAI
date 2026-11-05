/// Transport UI: play/stop, tempo factor, per-track mute, the playback cursor and MIDI
/// import/export. Imports only the `core_bridge` facade, `ui_kit` and `app_platform`
/// (FF-05). The Rust core owns every playback decision (GR-05, ADR-028).
library;

export 'src/midi_actions.dart'
    show ExportOutcome, ImportOutcome, MidiActions, midiActionsProvider;
export 'src/mute_control.dart' show MuteButton;
export 'src/playback_cursor.dart' show PlaybackCursor, TickToX;
export 'src/transport_bar.dart' show TransportBar;
export 'src/transport_controller.dart'
    show
        TransportController,
        TransportState,
        midiServiceProvider,
        playbackFactoryProvider,
        playbackServiceProvider,
        transportProvider;
