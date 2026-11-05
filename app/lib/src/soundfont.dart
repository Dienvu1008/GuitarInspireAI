/// Locates the bundled SoundFont for playback (ADR-026, slice 4c).
///
/// Slice 4c bundles `GuitarSteel.sf2` with the Windows app and resolves its path next to
/// the executable. Until then this returns `null`, which starts the engine with playback
/// disabled: the first Play reports a missing-SoundFont message and no sine fallback is
/// used (Q10). The seam is here so 4c only changes this one function.
Future<String?> bundledSoundFontPath() async {
  // TODO(4c): return the path of the bundled GuitarSteel.sf2 shipped with the app.
  return null;
}
