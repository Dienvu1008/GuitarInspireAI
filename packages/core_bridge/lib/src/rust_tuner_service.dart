import 'document_service.dart';
import 'generated/api/document.dart' show DocumentHandle;
import 'generated/api/tuner.dart' as ffi;
import 'generated/api/types.dart' show TunerReadingDto;
import 'rust_document_service.dart' show rustDocumentHandle;
import 'tuner_service.dart';

/// Starts a Rust-backed tuner bound to a [RustDocumentService]'s document.
final class RustTunerFactory implements TunerFactory {
  RustTunerFactory(this._document);

  final DocumentHandle _document;

  /// For the document behind [service] (must be Rust-backed).
  factory RustTunerFactory.forDocument(DocumentService service) =>
      RustTunerFactory(rustDocumentHandle(service));

  @override
  Future<TunerService> start() async {
    final handle = await ffi.TunerHandle.start();
    return RustTunerService._(handle, _document);
  }
}

final class RustTunerService implements TunerService {
  RustTunerService._(this._handle, this._document);

  final ffi.TunerHandle _handle;
  final DocumentHandle _document;
  bool _closed = false;

  @override
  TunerReadingDto reading(int track) =>
      _handle.reading(document: _document, track: track);

  @override
  bool get failed => _closed || _handle.failed();

  @override
  void close() {
    if (_closed) return;
    _closed = true;
    _handle.stop();
    _handle.dispose();
  }
}
