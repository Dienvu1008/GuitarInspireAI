// FF-05, FF-06, FF-08, FF-09 (Dart part).
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'dart_fitness.dart';

Directory _packages() {
  var d = Directory.current;
  while (!Directory('${d.path}/packages').existsSync()) {
    d = d.parent;
  }
  return Directory('${d.path}/packages');
}

Directory _app() => Directory.current.path.endsWith('app')
    ? Directory.current
    : Directory('${Directory.current.path}/app');

void main() {
  test('FF-05 Dart import boundaries', () {
    final errors = checkFF05(_packages());
    expect(errors, isEmpty, reason: errors.join('\n'));
  });

  test('FF-06 Dart package cycle check', () {
    final errors = checkFF06(_packages(), _app());
    expect(errors, isEmpty, reason: errors.join('\n'));
  });

  test('FF-08 dart:io only in app_platform', () {
    final errors = checkFF08(_packages());
    expect(errors, isEmpty, reason: errors.join('\n'));
  });

  test('FF-09 Dart: packages come from approved sources', () {
    final errors = checkFF09Dart(_packages(), _app());
    expect(errors, isEmpty, reason: errors.join('\n'));
  });
}
