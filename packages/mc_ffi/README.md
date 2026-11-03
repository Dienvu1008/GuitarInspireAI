# mc_ffi (native build plugin)
Builds the Rust crate `crates/mc-ffi` with [cargokit](cargokit/README) during the
Flutter build and bundles the resulting library with the app. It contains no Dart API;
the Dart facade is `packages/core_bridge`.
- `cargokit/` is vendored from flutter_rust_bridge 2.13.0
  (`flutter_rust_bridge_codegen integrate`); license: MIT or Apache-2.0
  (`cargokit/LICENSE`). It carries two local patches, both marked with a comment in
  the source; re-apply them if cargokit is ever re-vendored:
  1. `cargokit/cmake/cargokit.cmake` (Windows only): the manifest directory is built
     from the symlink-resolved `cargokit_cmake_root` (three levels up is the repo
     root) with PowerShell `GetFullPath`, instead of
     `CMAKE_CURRENT_SOURCE_DIR/<manifest_dir>`. Flutter loads the plugin through the
     directory symlink `app/windows/flutter/ephemeral/.plugin_symlinks/mc_ffi`, and
     CMake's `REALPATH`/`ABSOLUTE` do not follow Windows symlinks, so the upstream
     join produced a path that does not exist.
  2. `cargokit/build_tool/lib/src/artifacts_provider.dart`: local builds look for
     artifacts named after the lib target (`mc_ffi.dll`, package name with `-`
     replaced by `_`), not the raw package name (`mc-ffi.dll`). Upstream found no
     artifact, copied nothing and still exited 0, so the failure only showed up later
     as a missing file in the CMake INSTALL step.
- Manifest paths: `windows/CMakeLists.txt` passes `crates/mc-ffi` relative to the repo
  root (see patch 1). `linux/CMakeLists.txt` and the podspec keep upstream behaviour
  and use a path relative to the plugin symlink
  (`app/<platform>/flutter/ephemeral/.plugin_symlinks/mc_ffi/<platform>`); only
  Windows is a supported target in the MVP (Q-13).
- If a Windows build fails in INSTALL with a missing `build/native_assets/windows`
  directory, the Flutter build cache is stale; run `flutter clean` in `app/`.
