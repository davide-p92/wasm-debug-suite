cmake_minimum_required(VERSION 3.13)
project(dwarf_dump)

# Finde pkg-config
find_package(PkgConfig REQUIRED)

# Finde Zstd mit pkg-config
pkg_check_modules(ZSTD REQUIRED libzstd)

# Finde LLVM
find_package(LLVM REQUIRED CONFIG)

add_executable(dwarf_dump dwarf_dump.cpp)

# Setze Include-Verzeichnisse
target_include_directories(dwarf_dump PRIVATE 
  ${LLVM_INCLUDE_DIRS}
  ${ZSTD_INCLUDE_DIRS}
  ${CMAKE_SOURCE_DIR}/external
)

# Setze Linker-Optionen
target_link_directories(dwarf_dump PRIVATE ${ZSTD_LIBRARY_DIRS})
target_link_libraries(dwarf_dump PRIVATE 
  LLVMSupport 
  LLVMDebugInfoDWARF 
  LLVMObject 
  ${ZSTD_LIBRARIES}
)
