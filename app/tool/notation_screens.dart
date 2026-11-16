// ADR-041 section 9, F8: screenshots of the TAB view with notation, for review by
// eye. Opens each project, renders the view at 100 % and 200 % zoom offscreen and
// writes PNGs.
//
//   flutter build windows --profile -t tool/notation_screens.dart
//   build\windows\x64\runner\Profile\guitar_inspire.exe <out dir> <a.giproj> ...
import 'dart:io';
import 'dart:ui' as ui;

import 'package:core_bridge/core_bridge.dart';
import 'package:editor_feature/editor_feature.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';

const _width = 1400.0;
const _height = 1800.0;

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await CoreBridge.init();
  final container = ProviderContainer(
    overrides: [
      documentFactoryProvider.overrideWithValue(CoreBridge.documents),
    ],
  );
  final zoom = ValueNotifier(1.0);
  final boundary = GlobalKey();
  runApp(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: AppTheme.light(),
        home: Material(
          child: OverflowBox(
            alignment: Alignment.topLeft,
            minWidth: _width,
            maxWidth: _width,
            minHeight: _height,
            maxHeight: _height,
            child: RepaintBoundary(
              key: boundary,
              child: ValueListenableBuilder<double>(
                valueListenable: zoom,
                builder: (_, z, _) => TabView(zoom: z),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await WidgetsBinding.instance.endOfFrame;
  final out = args.first;
  for (final path in args.skip(1)) {
    final doc = await CoreBridge.documents.open(path);
    container.read(editorProvider.notifier).replace(doc);
    for (final z in [1.0, 2.0]) {
      zoom.value = z;
      // Two frames: the outline, then the systems it asks for.
      await WidgetsBinding.instance.endOfFrame;
      await WidgetsBinding.instance.endOfFrame;
      await Future<void>.delayed(const Duration(milliseconds: 300));
      final render =
          boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
      final image = await render.toImage();
      final png = await image.toByteData(format: ui.ImageByteFormat.png);
      final name = path.split(RegExp(r'[\\/]')).last.replaceAll('.giproj', '');
      final file = File('$out/$name-${(z * 100).round()}.png');
      await file.writeAsBytes(png!.buffer.asUint8List());
      stdout.writeln('wrote ${file.path}');
    }
  }
  exit(0);
}
