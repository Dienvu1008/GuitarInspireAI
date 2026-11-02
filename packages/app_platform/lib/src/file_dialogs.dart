import 'package:file_selector/file_selector.dart';

/// Open and save dialogs for project files. Returns `null` when the user cancels.
abstract interface class FileDialogs {
  Future<String?> pickProjectToOpen();

  /// [suggestedName] is shown in the dialog, for example `Untitled.giproj`.
  Future<String?> pickProjectToSave({required String suggestedName});
}

/// Product extension of project files (Q-08). Adapter-level convention only: the
/// core identifies files by their manifest, not by their name.
const String projectExtension = 'giproj';

const _projectFiles = XTypeGroup(
  label: 'GuitarInspire project',
  extensions: [projectExtension],
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
}
