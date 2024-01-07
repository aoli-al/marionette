add_library(InjectYieldPass SHARED
    InjectYieldPass.cc
)

set_target_properties(InjectYieldPass PROPERTIES
    COMPILE_FLAGS "-fno-rtti"
)

include_directories("${CMAKE_CURRENT_SOURCE_DIR}/../include")