cmake_minimum_required(VERSION 3.16)
project(wasm_debugger_project LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Wasmtime-Header und -Lib suchen
find_path(WASMTIME_INCLUDE_DIR wasmtime.h PATHS /usr/local/include)
find_library(WASMTIME_LIB wasmtime PATHS /usr/local/lib)

# Executable anlegen (erst hier existiert Target 'wasm_debugger')
add_executable(wasm_debugger
	wasm_debugger.cpp
	#tools/wasm_debugger/main.cpp        # falls vorhanden
)

# Include-Ordner anhängen
target_include_directories(wasm_debugger PRIVATE ${WASMTIME_INCLUDE_DIR})

# Bibliothek linken
target_link_libraries(wasm_debugger PRIVATE ${WASMTIME_LIB})

