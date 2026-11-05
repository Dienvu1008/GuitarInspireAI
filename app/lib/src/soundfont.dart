import 'dart:io';

/// Locates the bundled SoundFont for playback (ADR-026, slice 4c).
///
/// The Windows build copies `GuitarSteel.sf2` next to the executable (see
/// `app/windows/CMakeLists.txt`). At runtime it sits in the same directory as
/// `Platform.resolvedExecutable`, so that is where this looks. If the file is not there
/// (a platform without bundling yet, or a `flutter test` run whose executable is the
/// test host), it returns `null`: the engine then starts with playback disabled and the
/// first Play reports a missing-SoundFont message, with no sine fallback (Q10).
///
/// Only Windows is wired for the MVP (Q-13); other platforms return `null` until their
/// bundling step exists.
Future<String?> bundledSoundFontPath() async {
  if (!Platform.isWindows) return null;
  final exeDir = File(Platform.resolvedExecutable).parent.path;
  final candidate = '$exeDir${Platform.pathSeparator}$soundFontFileName';
  return File(candidate).existsSync() ? candidate : null;
}

/// File name of the bundled SoundFont, shared with the CMake install rule and tests.
const String soundFontFileName = 'GuitarSteel.sf2';
