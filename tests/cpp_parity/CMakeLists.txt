cmake_minimum_required(VERSION 3.20)
project(alembic_parity_test)

set(CMAKE_CXX_STANDARD 17)

find_package(Alembic REQUIRED)
find_package(Imath REQUIRED)

add_executable(test_xform test_xform.cpp)
target_link_libraries(test_xform PRIVATE Alembic::Alembic Imath::Imath)
