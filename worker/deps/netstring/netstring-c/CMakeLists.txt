cmake_minimum_required(VERSION 3.16)

set(PROJECT_NAME "netstring")
project(${PROJECT_NAME})

set(netstring_headers
  netstring.h
)
set(netstring_sources
  netstring.c
)

IF(NOT WIN32)
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wno-sign-compare")
ENDIF()

add_library(netstring
  STATIC
  ${netstring_sources}
  ${netstring_headers}
)

IF(APPLE)
    set_property(TARGET ${PROJECT_NAME}
        PROPERTY XCODE_ATTRIBUTE_WARNING_CFLAGS
        -Wall
        -Wno-sign-compare
    )
ENDIF()

target_include_directories(${PROJECT_NAME}
          INTERFACE ${CMAKE_CURRENT_SOURCE_DIR})

install(TARGETS ${PROJECT_NAME} DESTINATION lib)
install(FILES ${netstring_headers} DESTINATION include)
