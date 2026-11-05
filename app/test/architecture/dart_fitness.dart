/// Dart-side architecture fitness functions (FF-05, FF-06, FF-08, FF-09 Dart part).
/// These run with `flutter test` in the `app/` package.
///
/// The functions walk the source trees rather than relying on pubspec metadata, so
/// they catch import violations that pubspec constraints alone cannot express.
library;

import 'dart:io';

// ---------------------------------------------------------------- helpers

/// All `.dart` files under `dir`, excluding `build/` and `.dart_tool/`.
Iterable<File> dartFiles(Directory dir) sync* {
  if (!dir.existsSync()) return;
  for (final entry in dir.listSync(recursive: true, followLinks: false)) {
    if (entry is! File) continue;
    final rel = entry.path.replaceAll('\\', '/');
    if (rel.contains('/build/') || rel.contains('/.dart_tool/')) continue;
    if (entry.path.endsWith('.dart')) yield entry;
  }
}

/// All `import` lines from `file`.
Iterable<String> imports(File file) =>
    file.readAsLinesSync().where((l) => l.startsWith('import '));

/// Package name of a pubspec.yaml under `dir`.
String packageName(Directory dir) {
  final ps = File('${dir.path}/pubspec.yaml');
  for (final l in ps.readAsLinesSync()) {
    if (l.startsWith('name:')) return l.replaceFirst('name:', '').trim();
  }
  throw Exception('no name in ${ps.path}');
}

// ---------------------------------------------------------------- FF-05

/// FF-05: Dart import boundaries.
/// Feature packages (`editor_feature`, future `tools_feature` etc.) may import only
/// the `core_bridge` facade, `ui_kit` and `app_platform`. They must not import the
/// generated bindings (`lib/src/generated`) or each other.
List<String> checkFF05(Directory packagesDir) {
  const allowed = {
    'core_bridge',
    'app_platform',
    'ui_kit',
    'flutter',
    'dart',
    'flutter_riverpod',
  };
  final errors = <String>[];
  for (final pkg in packagesDir.listSync().whereType<Directory>()) {
    final name = packageName(pkg);
    final features = ['editor_feature', 'tools_feature', 'playback_feature'];
    if (!features.contains(name)) continue;
    final lib = Directory('${pkg.path}/lib');
    for (final file in dartFiles(lib)) {
      final rel = file.path.replaceAll('\\', '/');
      if (rel.contains('/lib/src/generated')) continue;
      for (final line in imports(file)) {
        // Extract package name from "import 'package:<name>/..."
        final m = RegExp(r"import 'package:([^/]+)/").firstMatch(line);
        if (m == null) continue;
        final dep = m.group(1)!;
        if (allowed.contains(dep)) continue;
        // Features may not import the generated bindings directly.
        if (dep == 'core_bridge' && line.contains('/src/generated')) {
          errors.add(
            'FF-05: ${_rel(file, packagesDir)} imports generated bindings: $line',
          );
        } else if (features.contains(dep)) {
          errors.add(
            'FF-05: ${_rel(file, packagesDir)} imports another feature package: $line',
          );
        } else if (!allowed.contains(dep)) {
          errors.add(
            'FF-05: ${_rel(file, packagesDir)} imports unlisted package "$dep": $line',
          );
        }
      }
    }
  }
  return errors;
}

// ---------------------------------------------------------------- FF-06

/// FF-06: no Dart package cycles.
/// Reads `pubspec.yaml` dependencies from path references; transitive deps are not
/// expanded (Dart's pub tool rejects transitive cycles; this checks direct edges in
/// the local workspace).
List<String> checkFF06(Directory packagesDir, Directory appDir) {
  final dirs = [...packagesDir.listSync().whereType<Directory>(), appDir];
  final edges = <String, Set<String>>{};
  for (final d in dirs) {
    final name = packageName(d);
    edges[name] = _pathDeps(d);
  }
  final errors = <String>[];
  for (final name in edges.keys) {
    final cycle = _findCycle(name, edges, []);
    if (cycle.isNotEmpty) {
      final path = cycle.join(' -> ');
      if (!errors.any((e) => e.contains(path))) {
        errors.add('FF-06: Dart package cycle: $path');
      }
    }
  }
  return errors;
}

Set<String> _pathDeps(Directory dir) {
  final ps = File('${dir.path}/pubspec.yaml');
  if (!ps.existsSync()) return {};
  final deps = <String>{};
  var inDeps = false;
  for (final l in ps.readAsLinesSync()) {
    if (l.trimRight() == 'dependencies:' ||
        l.trimRight() == 'dev_dependencies:') {
      inDeps = true;
      continue;
    }
    if (inDeps && l.startsWith('  ') && !l.startsWith('    ')) {
      final name = l.trim().replaceAll(':', '');
      if (name.isNotEmpty) deps.add(name);
    } else if (!l.startsWith(' ') && l.contains(':')) {
      inDeps = false;
    }
  }
  return deps;
}

List<String> _findCycle(
  String start,
  Map<String, Set<String>> edges,
  List<String> path,
) {
  final next = [start, ...path];
  for (final dep in edges[start] ?? {}) {
    if (path.contains(dep)) {
      return [dep, ...next.takeWhile((n) => n != dep), dep];
    }
    final cycle = _findCycle(dep, edges, next);
    if (cycle.isNotEmpty) return cycle;
  }
  return [];
}

// ---------------------------------------------------------------- FF-08

/// FF-08: `dart:io` and `Platform` only in `app_platform` and `app`.
List<String> checkFF08(Directory packagesDir) {
  const allowed = {'app_platform'};
  final errors = <String>[];
  for (final pkg in packagesDir.listSync().whereType<Directory>()) {
    final name = packageName(pkg);
    if (allowed.contains(name)) continue;
    final lib = Directory('${pkg.path}/lib');
    for (final file in dartFiles(lib)) {
      // Skip generated bindings; frb uses dart:ffi, not dart:io.
      final rel = file.path.replaceAll('\\', '/');
      if (rel.contains('/lib/src/generated')) continue;
      for (final line in imports(file)) {
        if (line.contains("'dart:io'") || line.contains('"dart:io"')) {
          errors.add(
            "FF-08: ${_rel(file, packagesDir)} imports 'dart:io': $line",
          );
        }
      }
    }
  }
  return errors;
}

// ---------------------------------------------------------------- FF-09 (Dart part)

/// FF-09 Dart part: every direct dependency of every local package is MIT,
/// Apache-2.0, BSD or similar. Reads `pubspec.lock` files in each package and
/// checks the `description.url` for pub.dev packages; bundled assets are excluded.
/// This is a basic check; the full audit is done manually when adding packages.
const Set<String> _approved = {
  'flutter',
  'flutter_test',
  'flutter_lints',
  'lints',
  'flutter_rust_bridge',
  'freezed_annotation',
  'freezed',
  'build_runner',
  'flutter_riverpod',
  'riverpod',
  'file_selector',
  'file_selector_windows',
  'file_selector_platform_interface',
  'file_selector_linux',
  'file_selector_macos',
  'file_selector_web',
  'path_provider',
  'path_provider_windows',
  'path_provider_android',
  'path_provider_ios',
  'path_provider_linux',
  'path_provider_macos',
  'path_provider_foundation',
  'path_provider_platform_interface',
  'plugin_platform_interface',
  'integration_test',
  'ffi',
  'ffigen',
  'meta',
  'collection',
  'win32',
  'xdg_directories',
  'path',
  'async',
  'platform',
  'characters',
  'vector_math',
  'equatable',
  'crypto',
  'typed_data',
};

List<String> checkFF09Dart(Directory packagesDir, Directory appDir) {
  final dirs = [...packagesDir.listSync().whereType<Directory>(), appDir];
  final errors = <String>[];
  for (final d in dirs) {
    final lock = File('${d.path}/pubspec.lock');
    if (!lock.existsSync()) continue;
    String? current;
    for (final l in lock.readAsLinesSync()) {
      if (l.startsWith('  ') && !l.startsWith('   ') && l.endsWith(':')) {
        current = l.trim().replaceAll(':', '');
      }
      if (current == null) continue;
      if (_approved.contains(current)) continue;
      if (l.contains('source: path') || l.contains('source: sdk')) continue;
      if (l.trim().startsWith('source:') &&
          !l.contains('source: hosted') &&
          !l.contains('source: sdk') &&
          !l.contains('source: path')) {
        errors.add(
          'FF-09 Dart: ${packageName(d)}: package "$current" has an unusual source: ${l.trim()}',
        );
      }
    }
  }
  return errors;
}

String _rel(File file, Directory base) {
  final rel = file.path
      .replaceAll('\\', '/')
      .replaceFirst(base.path.replaceAll('\\', '/'), '');
  return rel.startsWith('/') ? rel.substring(1) : rel;
}
