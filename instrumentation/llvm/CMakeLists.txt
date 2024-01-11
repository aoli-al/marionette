cmake_minimum_required(VERSION 3.4)
project(llvm-pass)


set (CMAKE_CXX_STANDARD 23)
set (CMAKE_EXPORT_COMPILE_COMMANDS 1)
set (CMAKE_BUILD_TYPE DEBUG)

find_package(LLVM 17 REQUIRED CONFIG)

if("${LLVM_VERSION_MAJOR}" VERSION_LESS 17)
  message(FATAL_ERROR "Found LLVM ${LLVM_VERSION_MAJOR}, but need LLVM 17 or above")
endif()


add_definitions(${LLVM_DEFINITIONS})
include_directories(${LLVM_INCLUDE_DIRS})
link_directories(${LLVM_LIBRARY_DIRS})



include(FetchContent)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG 123be1e3d8170c86e121392e8bffa4def7dc3447
)
FetchContent_MakeAvailable(Corrosion)


add_subdirectory(src)