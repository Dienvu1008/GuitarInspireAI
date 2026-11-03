import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Per-user folders of the application (security-and-privacy: autosave files stay in
/// per-user app data).
abstract interface class AppPaths {
  /// Folder for autosave and crash-recovery files; created if missing.
  Future<String> recoveryDirectory();
}

final class SystemAppPaths implements AppPaths {
  const SystemAppPaths();

  @override
  Future<String> recoveryDirectory() async {
    final base = await getApplicationSupportDirectory();
    final dir = Directory('${base.path}${Platform.pathSeparator}recovery');
    await dir.create(recursive: true);
    return dir.path;
  }
}
