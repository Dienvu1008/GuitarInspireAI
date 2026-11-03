cmake_minimum_required(VERSION 3.14)

set(PROJECT_NAME "mc_ffi")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
# Relative path from packages/mc_ffi/windows/ (the real location) to the
# Rust crate. cargokit.cmake (patched) builds this path from cargokit_cmake_root
# using PowerShell on Windows so that symlinks are handled correctly.
apply_cargokit(${PROJECT_NAME} "crates/mc-ffi" mc_ffi "")

set(mc_ffi_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
