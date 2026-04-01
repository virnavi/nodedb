cmake_minimum_required(VERSION 3.14)

project(nodedb_flutter_libs_library VERSION 0.0.1 LANGUAGES C)

set(NODEDB_DLL "${CMAKE_CURRENT_SOURCE_DIR}/nodedb_ffi.dll")

# Install the DLL into the Flutter bundle directory.
install(FILES ${NODEDB_DLL} DESTINATION "${CMAKE_INSTALL_PREFIX}/bin")
