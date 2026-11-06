/// Tuner, chord finder and fretboard UI. Imports only the `core_bridge` facade, `ui_kit`
/// and `app_platform` (FF-05). The Rust core owns pitch mapping and voicings (GR-05).
///
/// Slice 5d ships the tuner; 5e adds the chord finder and fretboard.
library;

export 'src/chord_controller.dart'
    show
        ChordController,
        ChordLookupFn,
        ChordState,
        InsertVoicingFn,
        chordLookupProvider,
        chordProvider,
        insertVoicingProvider;
export 'src/chord_finder_view.dart' show ChordFinderView;
export 'src/fretboard_view.dart' show FretboardView;
export 'src/tuner_controller.dart'
    show
        TunerController,
        TunerState,
        tunerFactoryProvider,
        tunerProvider,
        tunerTrackProvider;
export 'src/tuner_view.dart' show TunerView;
