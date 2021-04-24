cmake_minimum_required(VERSION 2.8)

project(libsrtp2 LANGUAGES C)

set(PACKAGE_VERSION 2.3.0)
set(PACKAGE_STRING "${CMAKE_PROJECT_NAME} ${PACKAGE_VERSION}")

include(TestBigEndian)
include(CheckIncludeFile)
include(CheckFunctionExists)
include(CheckTypeSize)
include(CheckCSourceCompiles)

test_big_endian(WORDS_BIGENDIAN)

if (CMAKE_SYSTEM_PROCESSOR MATCHES "(x86)|(X86)|(amd64)|(AMD64)")
  set (HAVE_X86 TRUE)
else ()
  set (HAVE_X86 FALSE)
endif ()

check_include_file(arpa/inet.h HAVE_ARPA_INET_H)
check_include_file(byteswap.h HAVE_BYTESWAP_H)
check_include_file(inttypes.h HAVE_INTTYPES_H)
check_include_file(machine/types.h HAVE_MACHINE_TYPES_H)
check_include_file(netinet/in.h HAVE_NETINET_IN_H)
check_include_file(stdint.h HAVE_STDINT_H)
check_include_file(stdlib.h HAVE_STDLIB_H)
check_include_file(sys/int_types.h HAVE_SYS_INT_TYPES_H)
check_include_file(sys/socket.h HAVE_SYS_SOCKET_H)
check_include_file(sys/types.h HAVE_SYS_TYPES_H)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(windows.h HAVE_WINDOWS_H)
check_include_file(winsock2.h HAVE_WINSOCK2_H)

check_function_exists(sigaction HAVE_SIGACTION)
check_function_exists(inet_aton HAVE_INET_ATON)
check_function_exists(usleep HAVE_USLEEP)

check_type_size(uint8_t UINT8_T)
check_type_size(uint16_t UINT16_T)
check_type_size(uint32_t UINT32_T)
check_type_size(uint64_t UINT64_T)
check_type_size(int32_t INT32_T)
check_type_size("unsigned long" SIZEOF_UNSIGNED_LONG)
check_type_size("unsigned long long" SIZEOF_UNSIGNED_LONG_LONG)

check_c_source_compiles("inline void func(); void func() { } int main() { func(); return 0; }" HAVE_INLINE)
if(NOT HAVE_INLINE)
  check_c_source_compiles("__inline void func(); void func() { } int main() { func(); return 0; }" HAVE___INLINE)
endif()

set(ENABLE_DEBUG_LOGGING OFF CACHE BOOL "Enable debug logging in all modules")
set(ERR_REPORTING_STDOUT OFF CACHE BOOL "Enable logging to stdout")
set(ERR_REPORTING_FILE "" CACHE FILEPATH "Use file for logging")
set(ENABLE_OPENSSL OFF CACHE BOOL "Enable OpenSSL crypto engine")
set(TEST_APPS ON CACHE BOOL "Build test applications")
option(BUILD_SHARED_LIBS "Build shared library" OFF)

if(ENABLE_OPENSSL)
  #find_package(OpenSSL REQUIRED)
  #include_directories(${OPENSSL_INCLUDE_DIR})
endif()
set(OPENSSL ${ENABLE_OPENSSL} CACHE BOOL INTERNAL)
set(GCM ${ENABLE_OPENSSL} CACHE BOOL INTERNAL)

set(CONFIG_FILE_DIR ${CMAKE_CURRENT_BINARY_DIR})
include_directories(${CONFIG_FILE_DIR})

configure_file(config_in_cmake.h ${CONFIG_FILE_DIR}/config.h)
add_definitions(-DHAVE_CONFIG_H)

set(SOURCES_C
  srtp/ekt.c
  srtp/srtp.c
)

set(CIPHERS_SOURCES_C
  crypto/cipher/cipher.c
  crypto/cipher/null_cipher.c
)

if(ENABLE_OPENSSL)
  list(APPEND CIPHERS_SOURCES_C
    crypto/cipher/aes_icm_ossl.c
    crypto/cipher/aes_gcm_ossl.c
  )
else()
  list(APPEND  CIPHERS_SOURCES_C
    crypto/cipher/aes.c
    crypto/cipher/aes_icm.c
  )
endif()

set(HASHES_SOURCES_C
    crypto/hash/auth.c
    crypto/hash/null_auth.c
)

if(ENABLE_OPENSSL)
  list(APPEND HASHES_SOURCES_C
    crypto/hash/hmac_ossl.c
  )
else()
  list(APPEND  HASHES_SOURCES_C
    crypto/hash/hmac.c
    crypto/hash/sha1.c
  )
endif()

set(KERNEL_SOURCES_C
  crypto/kernel/alloc.c
  crypto/kernel/crypto_kernel.c
  crypto/kernel/err.c
  crypto/kernel/key.c
)

set(MATH_SOURCES_C
  crypto/math/datatypes.c
  crypto/math/stat.c
)

set(REPLAY_SOURCES_C
  crypto/replay/rdb.c
  crypto/replay/rdbx.c
  crypto/replay/ut_sim.c
)

set(SOURCES_H
  crypto/include/aes.h
  crypto/include/aes_icm.h
  crypto/include/alloc.h
  crypto/include/auth.h
  crypto/include/cipher.h
  crypto/include/cipher_types.h
  crypto/include/crypto_kernel.h
  crypto/include/crypto_types.h
  crypto/include/datatypes.h
  crypto/include/err.h
  crypto/include/hmac.h
  crypto/include/integers.h
  crypto/include/key.h
  crypto/include/null_auth.h
  crypto/include/null_cipher.h
  crypto/include/rdb.h
  crypto/include/rdbx.h
  crypto/include/sha1.h
  crypto/include/stat.h
  include/srtp.h
  include/srtp_priv.h
  include/ut_sim.h
  ${CONFIG_FILE_DIR}/config.h
)

if(BUILD_SHARED_LIBS AND WIN32)
  list(APPEND SOURCES_C
    srtp.def
  )
endif()

source_group("src" FILES ${SOURCES_C})
source_group("src\\Ciphers" FILES ${CIPHERS_SOURCES_C})
source_group("src\\Hashes" FILES ${HASHES_SOURCES_C})
source_group("src\\Kernel" FILES ${KERNEL_SOURCES_C})
source_group("src\\Math" FILES ${MATH_SOURCES_C})
source_group("src\\Replay" FILES ${REPLAY_SOURCES_C})
source_group("include" FILES ${SOURCES_H})

add_library(srtp2
  ${SOURCES_C}
  ${CIPHERS_SOURCES_C}
  ${HASHES_SOURCES_C}
  ${KERNEL_SOURCES_C}
  ${MATH_SOURCES_C}
  ${REPLAY_SOURCES_C}
  ${SOURCES_H}
)

target_include_directories(srtp2 PUBLIC crypto/include include)
if(ENABLE_OPENSSL)
  target_link_libraries(srtp2 crypto)
  target_compile_definitions(srtp2 PRIVATE "OPENSSL" "GCM")
endif()
if(WIN32)
  target_link_libraries(srtp2 ws2_32)
endif()

install(TARGETS srtp2 DESTINATION lib)
install(FILES include/srtp.h crypto/include/auth.h
  crypto/include/cipher.h
  crypto/include/cipher_types.h
  DESTINATION include/srtp2)

if(TEST_APPS)
  enable_testing()

if(NOT (BUILD_SHARED_LIBS AND WIN32))
  add_executable(test_srtp test/test_srtp.c)
  target_link_libraries(test_srtp srtp2)
  add_test(test_srtp test_srtp)
endif()

  add_executable(srtp_driver test/srtp_driver.c
    test/util.c test/getopt_s.c)
  target_link_libraries(srtp_driver srtp2)
  add_test(srtp_driver srtp_driver -v)
endif()
