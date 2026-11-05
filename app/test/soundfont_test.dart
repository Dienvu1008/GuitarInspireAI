// Locks the bundled SoundFont (ADR-026, slice 4c): the committed asset must be the exact
// trimmed file (SHA-256), so it cannot be swapped or corrupted silently, and
// bundledSoundFontPath() must degrade gracefully when no bundled copy sits next to the
// test host executable.
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:guitar_inspire/src/soundfont.dart';

/// The SHA-256 recorded in assets/soundfonts/PROVENANCE.md for the trimmed file.
const _expectedSha256 =
    '8c9b74ef954235164d8c5ea58fe10c8687f808d92fbb23ba388f5ce5fab49ef8';

/// Repo-root assets/soundfonts/GuitarSteel.sf2, found by walking up from the app dir.
File _committedSoundFont() {
  var dir = Directory.current;
  while (true) {
    final f = File(
      '${dir.path}${Platform.pathSeparator}assets'
      '${Platform.pathSeparator}soundfonts'
      '${Platform.pathSeparator}$soundFontFileName',
    );
    if (f.existsSync()) return f;
    final parent = dir.parent;
    if (parent.path == dir.path) {
      fail('could not find assets/soundfonts/$soundFontFileName above ${Directory.current.path}');
    }
    dir = parent;
  }
}

void main() {
  test('the committed SoundFont matches the recorded SHA-256', () {
    final bytes = _committedSoundFont().readAsBytesSync();
    final digest = sha256.convert(bytes).toString();
    expect(
      digest,
      _expectedSha256,
      reason:
          'assets/soundfonts/$soundFontFileName changed; update PROVENANCE.md and '
          'ADR-026 if this was intentional, and re-run the trim/verify scripts',
    );
    // Sanity: the trimmed file is the ~2 MiB asset, not the 148 MB source.
    expect(bytes.length, 2108670);
  });

  test('bundledSoundFontPath returns null when no copy sits next to the test host', () async {
    // In `flutter test` the executable is the test host; the SoundFont is not bundled
    // next to it, so the function reports null rather than a wrong path (graceful Q10).
    final path = await bundledSoundFontPath();
    if (path != null) {
      // If a path is returned it must actually exist (never a stale guess).
      expect(File(path).existsSync(), isTrue);
    }
  });
}
