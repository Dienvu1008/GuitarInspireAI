import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';

/// Open and save dialogs for project, MIDI and MusicXML files. Returns `null` when the user
/// cancels. MIDI bytes are read and written here because file I/O is a platform concern
/// (GR-11, FF-08); the core only receives and produces bytes.
abstract interface class FileDialogs {
  Future<String?> pickProjectToOpen();

  /// [suggestedName] is shown in the dialog, for example `Untitled.giproj`.
  Future<String?> pickProjectToSave({required String suggestedName});

  /// Pick a Standard MIDI File and return its bytes, or `null` if cancelled.
  Future<List<int>?> pickMidiToImport();

  /// Pick a save location and write [bytes] as a `.mid` file. Returns the path written,
  /// or `null` if cancelled.
  Future<String?> saveMidi(List<int> bytes, {required String suggestedName});

  /// Pick a save location and write [bytes] as a `.musicxml` file (ADR-033). Returns
  /// the path written, or `null` if cancelled.
  Future<String?> saveMusicXml(
    List<int> bytes, {
    required String suggestedName,
  });
}

/// Product extension of project files (Q-08). Adapter-level convention only: the
/// core identifies files by their manifest, not by their name.
const String projectExtension = 'giproj';

const _projectFiles = XTypeGroup(
  label: 'GuitarInspire project',
  extensions: [projectExtension],
);

const _musicXmlFiles = XTypeGroup(label: 'MusicXML', extensions: ['musicxml']);

const _midiFiles = XTypeGroup(
  label: 'Standard MIDI File',
  extensions: ['mid', 'midi'],
);

/// System dialogs via the `file_selector` plugin.
final class SystemFileDialogs implements FileDialogs {
  const SystemFileDialogs();

  @override
  Future<String?> pickProjectToOpen() async {
    final file = await openFile(acceptedTypeGroups: const [_projectFiles]);
    return file?.path;
  }

  @override
  Future<String?> pickProjectToSave({required String suggestedName}) async {
    final location = await getSaveLocation(
      acceptedTypeGroups: const [_projectFiles],
      suggestedName: suggestedName,
    );
    if (location == null) return null;
    final path = location.path;
    // Append the product extension when the user typed a name without one.
    final name = path.split(RegExp(r'[\\/]')).last;
    return name.contains('.') ? path : '$path.$projectExtension';
  }

  @override
  Future<List<int>?> pickMidiToImport() async {
    final file = await openFile(acceptedTypeGroups: const [_midiFiles]);
    if (file == null) return null;
    return file.readAsBytes();
  }

  @override
  Future<String?> saveMidi(List<int> bytes, {required String suggestedName}) =>
      _save(bytes, suggestedName, _midiFiles, 'mid');

  @override
  Future<String?> saveMusicXml(
    List<int> bytes, {
    required String suggestedName,
  }) => _save(bytes, suggestedName, _musicXmlFiles, 'musicxml');

  /// Ask for a location and write [bytes] there, appending [extension] when the user
  /// typed a name without one.
  Future<String?> _save(
    List<int> bytes,
    String suggestedName,
    XTypeGroup type,
    String extension,
  ) async {
    final location = await getSaveLocation(
      acceptedTypeGroups: [type],
      suggestedName: suggestedName,
    );
    if (location == null) return null;
    final name = location.path.split(RegExp(r'[\\/]')).last;
    final path = name.contains('.')
        ? location.path
        : '${location.path}.$extension';
    await XFile.fromData(
      Uint8List.fromList(bytes),
      name: suggestedName,
    ).saveTo(path);
    return path;
  }
}
