# //! Fcitx4 壳；输入与词库逻辑仍由 Linux Server 负责。
cmake_minimum_required(VERSION 3.16)
project(qingjian-fcitx4 LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
find_package(PkgConfig REQUIRED)
pkg_check_modules(FCITX4 REQUIRED IMPORTED_TARGET fcitx)
pkg_check_modules(X11 REQUIRED IMPORTED_TARGET x11)
pkg_check_modules(XRES REQUIRED IMPORTED_TARGET xres)
find_package(nlohmann_json 3.2 REQUIRED)
include(CTest)
add_library(fcitx-qingjian MODULE
    src/qingjian.cpp
    src/key.cpp
    src/primary_selection.cpp
    ../fcitx5/src/ipc/connection.cpp)
target_include_directories(fcitx-qingjian PRIVATE src ../fcitx5/src/ipc)
target_link_libraries(fcitx-qingjian PRIVATE PkgConfig::FCITX4 PkgConfig::X11 PkgConfig::XRES nlohmann_json::nlohmann_json)
set_target_properties(fcitx-qingjian PROPERTIES PREFIX "")
if(BUILD_TESTING)
    add_executable(test-surrounding ../tests/surrounding.cpp)
    target_link_libraries(test-surrounding PRIVATE nlohmann_json::nlohmann_json)
    add_test(NAME surrounding COMMAND test-surrounding)
endif()
