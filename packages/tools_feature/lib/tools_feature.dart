/// Tuner, chord finder and fretboard UI. Imports only the `core_bridge` facade, `ui_kit`
/// and `app_platform` (FF-05). The Rust core owns pitch mapping and voicings (GR-05).
///
/// Slice 5d ships the tuner; the chord finder and fretboard land in 5e.
library;

export 'src/tuner_controller.dart'
    show
        TunerController,
        TunerState,
        tunerFactoryProvider,
        tunerProvider,
        tunerTrackProvider;
export 'src/tuner_view.dart' show TunerView;
