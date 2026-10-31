# mc_ffi (native build plugin)

Builds the Rust crate `crates/mc-ffi` with [cargokit](cargokit/README) during the
Flutter build and bundles the resulting library with the app. It contains no Dart API;
the Dart facade is `packages/core_bridge`.

- `cargokit/` is vendored unmodified from flutter_rust_bridge 2.13.0
  (`flutter_rust_bridge_codegen integrate`); license: MIT or Apache-2.0
  (`cargokit/LICENSE`).
- The manifest path in `windows/CMakeLists.txt`, `linux/CMakeLists.txt` and the
  podspec is relative to the plugin symlink that Flutter creates under the app
  (`app/<platform>/flutter/ephemeral/.plugin_symlinks/mc_ffi/<platform>`).
