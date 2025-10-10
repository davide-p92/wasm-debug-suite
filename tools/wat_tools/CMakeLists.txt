cmake_minimum_required(VERSION 3.14)
project(wat_tools)

# Wabt als Subdir einbinden
find_package(wabt REQUIRED)

# Mein Executable
add_executable(wat_tools wat_tools.cpp)
target_link_libraries(wat_tools PRIVATE wabt::wabt)

# Zusätzlicher Include-Pfad für interne Header
target_include_directories(wat_tools PRIVATE
	${CMAKE_SOURCE_DIR}/../../wabt/src
	${CMAKE_SOURCE_DIR}/../../wabt/include
)
