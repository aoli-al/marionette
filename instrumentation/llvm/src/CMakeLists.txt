add_library(InjectYieldPass SHARED
    InjectYieldPass.cc
)

set_target_properties(InjectYieldPass PROPERTIES
    COMPILE_FLAGS "-fno-rtti"
)

include_directories("${CMAKE_CURRENT_SOURCE_DIR}/../include")

corrosion_import_crate(MANIFEST_PATH ${CMAKE_SOURCE_DIR}/rust/rustc-demangle/Cargo.toml)
corrosion_experimental_cbindgen(
        TARGET rustc_demangle
        CARGO_PACKAGE rustc-demangle-capi
        HEADER_NAME rust_demangle.h
        MANIFEST_DIRECTORY ${CMAKE_SOURCE_DIR}/rust/rustc-demangle
)
target_link_libraries(InjectYieldPass rustc_demangle)
target_include_directories(InjectYieldPass
    PRIVATE
    "$<TARGET_PROPERTY:rustc_demangle,INTERFACE_INCLUDE_DIRECTORIES>"
)

add_dependencies(InjectYieldPass rustc_demangle)
target_link_options( PRIVATE "LINKER:-as-needed")