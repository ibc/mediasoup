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

file( READ ${CMAKE_CURRENT_SOURCE_DIR}/CA.pl.cmake CA_PL )
file( WRITE ${CMAKE_CURRENT_BINARY_DIR}/CA.pl "${CA_PL}" )

file( READ ${CMAKE_CURRENT_SOURCE_DIR}/tsget.cmake TSGET )
string( REPLACE "@VERSION_STRING@" "${VERSION_STRING}" TSGET "${TSGET}" )
file( WRITE ${CMAKE_CURRENT_BINARY_DIR}/tsget "${TSGET}" )

file( READ ${CMAKE_CURRENT_SOURCE_DIR}/progs.h.cmake PROGS_H )
file( WRITE ${CMAKE_CURRENT_BINARY_DIR}/progs.h "${PROGS_H}" )

add_definitions( -DMONOLITH )

set( E_SRC
  app_rand.c apps.c asn1pars.c bf_prefix.c ca.c ciphers.c cms.c crl.c crl2p7.c dgst.c dhparam.c
  dsa.c dsaparam.c ec.c ecparam.c enc.c engine.c errstr.c gendsa.c genpkey.c
  genrsa.c nseq.c ocsp.c openssl.c opt.c passwd.c pkcs12.c pkcs7.c pkcs8.c pkey.c
  pkeyparam.c pkeyutl.c prime.c rand.c rehash.c req.c rsa.c rsautl.c s_cb.c
  s_client.c s_server.c s_socket.c s_time.c sess_id.c smime.c speed.c spkac.c storeutl.c srp.c
  ts.c verify.c version.c x509.c )

include_directories( BEFORE SYSTEM
  ${PROJECT_SOURCE_DIR}/ # e_os.h
  ${CMAKE_CURRENT_BINARY_DIR}/ # progs.h
)

if( WIN32 )
  list( APPEND E_SRC win32_init.c )
endif()

add_executable( openssl ${E_SRC} )

target_link_libraries( openssl crypto ssl )

install( TARGETS openssl
    RUNTIME DESTINATION bin )

install( FILES
  ${CMAKE_CURRENT_BINARY_DIR}/CA.pl
  ${CMAKE_CURRENT_BINARY_DIR}/tsget
  DESTINATION share/openssl )
