cmake_minimum_required(VERSION 3.10)

project(nodedb_flutter_libs_library VERSION 0.0.1 LANGUAGES C)

# This is the shared library that Flutter will bundle.
set(NODEDB_LIB "${CMAKE_CURRENT_SOURCE_DIR}/libnodedb_ffi.so")

# Install the shared library into the Flutter bundle directory.
install(FILES ${NODEDB_LIB} DESTINATION "${CMAKE_INSTALL_PREFIX}/lib")
