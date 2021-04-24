# Based on original work by David Manura
# Copyright (C) 2007-2012 LuaDist.
# Copyright (C) 2013 Brian Sidebotham

# Redistribution and use of this file is allowed according to the terms of the
# MIT license.
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

if( BUILD_SHARED_LIBS )
  add_definitions( -DOPENSSL_BUILD_SHLIBSSL )
endif()

set( LIBSRC
  bio_ssl.c d1_lib.c d1_msg.c d1_srtp.c methods.c packet.c pqueue.c s3_cbc.c s3_enc.c s3_lib.c
  s3_msg.c ssl_asn1.c ssl_cert.c ssl_ciph.c ssl_conf.c ssl_err.c ssl_init.c ssl_lib.c
  ssl_mcnf.c ssl_rsa.c ssl_sess.c ssl_stat.c ssl_txt.c ssl_utst.c t1_enc.c t1_lib.c
  t1_trce.c tls13_enc.c tls_srp.c
  record/dtls1_bitmap.c record/rec_layer_d1.c record/rec_layer_s3.c record/ssl3_buffer.c
  record/ssl3_record.c record/ssl3_record_tls13.c
  statem/extensions.c statem/extensions_clnt.c statem/extensions_cust.c
  statem/extensions_srvr.c statem/statem.c statem/statem_clnt.c statem/statem_dtls.c
  statem/statem_lib.c statem/statem_srvr.c
  )

include_directories( BEFORE SYSTEM
  ${PROJECT_SOURCE_DIR}/ # e_os.h
)

add_library( ssl ${LIBSRC} )

target_include_directories( ssl PUBLIC ${PROJECT_BINARY_DIR}/include )

target_link_libraries( ssl crypto )

set_target_properties( ssl PROPERTIES
  VERSION "${LIB_VERSION}" SOVERSION "${LIB_SOVERSION}" )

if( ANDROID )
  set_target_properties( ssl PROPERTIES OUTPUT_NAME "ssl_1_1" )
endif()

if( MSVC )
  if( CMAKE_SIZEOF_VOID_P EQUAL 8 )
    set_target_properties( ssl PROPERTIES OUTPUT_NAME "ssl-1_1-x64" )
  elseif( CMAKE_SIZEOF_VOID_P EQUAL 4 )
    set_target_properties( ssl PROPERTIES OUTPUT_NAME "ssl-1_1" )
  endif()
endif()

install( TARGETS ssl
    RUNTIME DESTINATION bin
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib )
