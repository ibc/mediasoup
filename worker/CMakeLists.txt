cmake_minimum_required(VERSION 3.16)

include (TestBigEndian)
TEST_BIG_ENDIAN(IS_BIG_ENDIAN)

# add subdirectory

add_subdirectory(deps/libuv/libuv)

# add openssl no need app 
set(WITH_APPS OFF CACHE INTERNAL "")
add_subdirectory(deps/openssl/openssl)
unset(WITH_APPS)

add_subdirectory(deps/libwebrtc)
add_subdirectory(deps/catch)

set(TEST_APPS OFF CACHE INTERNAL "")
set(ENABLE_OPENSSL ON CACHE INTERNAL "")
add_subdirectory(deps/libsrtp/srtp)
unset(TEST_APPS)
unset(ENABLE_OPENSSL)

add_subdirectory(deps/netstring/netstring-c)

set(sctp_werror OFF CACHE INTERNAL "")
set(sctp_build_programs OFF CACHE INTERNAL "")
set(sctp_build_fuzzer OFF CACHE INTERNAL "")
set(sctp_link_programs_static OFF CACHE INTERNAL "")
add_subdirectory(deps/usrsctp/usrsctp)
unset(sctp_werror)
unset(sctp_build_programs)
unset(sctp_build_fuzzer)
unset(sctp_link_programs_static)

set(JSON_BuildTests OFF CACHE INTERNAL "")
set(JSON_Install OFF CACHE INTERNAL "")
add_subdirectory(deps/json)
unset(JSON_BuildTests)
unset(JSON_Install)

IF(WIN32)
add_subdirectory(deps/getopt/getopt)
ENDIF()

# mediasoup-worker

set(PROJECT_NAME "mediasoup-worker")
project(${PROJECT_NAME})

set(mediasoup_worker_headers
    "include/DepLibSRTP.hpp"
    "include/DepLibUV.hpp"
    "include/DepLibWebRTC.hpp"
    "include/DepOpenSSL.hpp"
    "include/DepUsrSCTP.hpp"
    "include/LogLevel.hpp"
    "include/Logger.hpp"
    "include/MediaSoupErrors.hpp"
    "include/Settings.hpp"
    "include/Utils.hpp"
    "include/Worker.hpp"
    "include/common.hpp"
    "include/handles/SignalsHandler.hpp"
    "include/handles/TcpConnection.hpp"
    "include/handles/TcpServer.hpp"
    "include/handles/Timer.hpp"
    "include/handles/UdpSocket.hpp"
    "include/handles/UnixStreamSocket.hpp"
    "include/Channel/Notifier.hpp"
    "include/Channel/Request.hpp"
    "include/Channel/UnixStreamSocket.hpp"
    "include/PayloadChannel/Notifier.hpp"
    "include/PayloadChannel/Notification.hpp"
    "include/PayloadChannel/UnixStreamSocket.hpp"
    "include/PayloadChannel/Request.hpp"
    "include/RTC/BweType.hpp"
    "include/RTC/AudioLevelObserver.hpp"
    "include/RTC/Consumer.hpp"
    "include/RTC/DataConsumer.hpp"
    "include/RTC/DataProducer.hpp"
    "include/RTC/DtlsTransport.hpp"
    "include/RTC/IceCandidate.hpp"
    "include/RTC/IceServer.hpp"
    "include/RTC/KeyFrameRequestManager.hpp"
    "include/RTC/NackGenerator.hpp"
    "include/RTC/Parameters.hpp"
    "include/RTC/PipeConsumer.hpp"
    "include/RTC/PipeTransport.hpp"
    "include/RTC/PlainTransport.hpp"
    "include/RTC/PortManager.hpp"
    "include/RTC/Producer.hpp"
    "include/RTC/RateCalculator.hpp"
    "include/RTC/Router.hpp"
    "include/RTC/RtpDictionaries.hpp"
    "include/RTC/RtpHeaderExtensionIds.hpp"
    "include/RTC/RtpListener.hpp"
    "include/RTC/RtpObserver.hpp"
    "include/RTC/RtpPacket.hpp"
    "include/RTC/RtpProbationGenerator.hpp"
    "include/RTC/RtpStream.hpp"
    "include/RTC/RtpStreamRecv.hpp"
    "include/RTC/RtpStreamSend.hpp"
    "include/RTC/RtxStream.hpp"
    "include/RTC/SctpAssociation.hpp"
    "include/RTC/SctpDictionaries.hpp"
    "include/RTC/SctpListener.hpp"
    "include/RTC/SenderBandwidthEstimator.hpp"
    "include/RTC/SeqManager.hpp"
    "include/RTC/SimpleConsumer.hpp"
    "include/RTC/SimulcastConsumer.hpp"
    "include/RTC/SrtpSession.hpp"
    "include/RTC/StunPacket.hpp"
    "include/RTC/SvcConsumer.hpp"
    "include/RTC/TcpConnection.hpp"
    "include/RTC/TcpServer.hpp"
    "include/RTC/Transport.hpp"
    "include/RTC/TransportCongestionControlClient.hpp"
    "include/RTC/TransportCongestionControlServer.hpp"
    "include/RTC/TransportTuple.hpp"
    "include/RTC/TrendCalculator.hpp"
    "include/RTC/UdpSocket.hpp"
    "include/RTC/WebRtcTransport.hpp"
    "include/RTC/DirectTransport.hpp"
    "include/RTC/Codecs/Tools.hpp"
    "include/RTC/Codecs/PayloadDescriptorHandler.hpp"
    "include/RTC/Codecs/H264.hpp"
    "include/RTC/Codecs/VP8.hpp"
    "include/RTC/Codecs/VP9.hpp"
    "include/RTC/RTCP/Packet.hpp"
    "include/RTC/RTCP/CompoundPacket.hpp"
    "include/RTC/RTCP/SenderReport.hpp"
    "include/RTC/RTCP/ReceiverReport.hpp"
    "include/RTC/RTCP/Sdes.hpp"
    "include/RTC/RTCP/Bye.hpp"
    "include/RTC/RTCP/Feedback.hpp"
    "include/RTC/RTCP/FeedbackItem.hpp"
    "include/RTC/RTCP/FeedbackPs.hpp"
    "include/RTC/RTCP/FeedbackRtp.hpp"
    "include/RTC/RTCP/FeedbackRtpNack.hpp"
    "include/RTC/RTCP/FeedbackRtpTmmb.hpp"
    "include/RTC/RTCP/FeedbackRtpSrReq.hpp"
    "include/RTC/RTCP/FeedbackRtpTllei.hpp"
    "include/RTC/RTCP/FeedbackRtpEcn.hpp"
    "include/RTC/RTCP/FeedbackRtpTransport.hpp"
    "include/RTC/RTCP/FeedbackPsPli.hpp"
    "include/RTC/RTCP/FeedbackPsSli.hpp"
    "include/RTC/RTCP/FeedbackPsRpsi.hpp"
    "include/RTC/RTCP/FeedbackPsFir.hpp"
    "include/RTC/RTCP/FeedbackPsTst.hpp"
    "include/RTC/RTCP/FeedbackPsVbcm.hpp"
    "include/RTC/RTCP/FeedbackPsLei.hpp"
    "include/RTC/RTCP/FeedbackPsAfb.hpp"
    "include/RTC/RTCP/FeedbackPsRemb.hpp"
    "include/RTC/RTCP/XR.hpp"
    "include/RTC/RTCP/XrDelaySinceLastRr.hpp"
    "include/RTC/RTCP/XrReceiverReferenceTime.hpp"
)

set(mediasoup_worker_sources
    "src/main.cpp"
    "src/lib.cpp"
    "src/DepLibSRTP.cpp"
    "src/DepLibUV.cpp"
    "src/DepLibWebRTC.cpp"
    "src/DepOpenSSL.cpp"
    "src/DepUsrSCTP.cpp"
    "src/Logger.cpp"
    "src/Settings.cpp"
    "src/Worker.cpp"
    "src/Utils/Crypto.cpp"
    "src/Utils/File.cpp"
    "src/Utils/IP.cpp"
    "src/Utils/String.cpp"
    "src/handles/SignalsHandler.cpp"
    "src/handles/TcpConnection.cpp"
    "src/handles/TcpServer.cpp"
    "src/handles/Timer.cpp"
    "src/handles/UdpSocket.cpp"
    "src/handles/UnixStreamSocket.cpp"
    "src/Channel/Notifier.cpp"
    "src/Channel/Request.cpp"
    "src/Channel/UnixStreamSocket.cpp"
    "src/PayloadChannel/Notifier.cpp"
    "src/PayloadChannel/Notification.cpp"
    "src/PayloadChannel/UnixStreamSocket.cpp"
    "src/PayloadChannel/Request.cpp"
    "src/RTC/AudioLevelObserver.cpp"
    "src/RTC/Consumer.cpp"
    "src/RTC/DataConsumer.cpp"
    "src/RTC/DataProducer.cpp"
    "src/RTC/DtlsTransport.cpp"
    "src/RTC/IceCandidate.cpp"
    "src/RTC/IceServer.cpp"
    "src/RTC/KeyFrameRequestManager.cpp"
    "src/RTC/NackGenerator.cpp"
    "src/RTC/PipeConsumer.cpp"
    "src/RTC/PipeTransport.cpp"
    "src/RTC/PlainTransport.cpp"
    "src/RTC/PortManager.cpp"
    "src/RTC/Producer.cpp"
    "src/RTC/RateCalculator.cpp"
    "src/RTC/Router.cpp"
    "src/RTC/RtpListener.cpp"
    "src/RTC/RtpObserver.cpp"
    "src/RTC/RtpPacket.cpp"
    "src/RTC/RtpProbationGenerator.cpp"
    "src/RTC/RtpStream.cpp"
    "src/RTC/RtpStreamRecv.cpp"
    "src/RTC/RtpStreamSend.cpp"
    "src/RTC/RtxStream.cpp"
    "src/RTC/SctpAssociation.cpp"
    "src/RTC/SctpListener.cpp"
    "src/RTC/SenderBandwidthEstimator.cpp"
    "src/RTC/SeqManager.cpp"
    "src/RTC/SimpleConsumer.cpp"
    "src/RTC/SimulcastConsumer.cpp"
    "src/RTC/SrtpSession.cpp"
    "src/RTC/StunPacket.cpp"
    "src/RTC/SvcConsumer.cpp"
    "src/RTC/TcpConnection.cpp"
    "src/RTC/TcpServer.cpp"
    "src/RTC/Transport.cpp"
    "src/RTC/TransportCongestionControlClient.cpp"
    "src/RTC/TransportCongestionControlServer.cpp"
    "src/RTC/TransportTuple.cpp"
    "src/RTC/TrendCalculator.cpp"
    "src/RTC/UdpSocket.cpp"
    "src/RTC/WebRtcTransport.cpp"
    "src/RTC/DirectTransport.cpp"
    "src/RTC/Codecs/H264.cpp"
    "src/RTC/Codecs/VP8.cpp"
    "src/RTC/Codecs/VP9.cpp"
    "src/RTC/RtpDictionaries/Media.cpp"
    "src/RTC/RtpDictionaries/Parameters.cpp"
    "src/RTC/RtpDictionaries/RtcpFeedback.cpp"
    "src/RTC/RtpDictionaries/RtcpParameters.cpp"
    "src/RTC/RtpDictionaries/RtpCodecMimeType.cpp"
    "src/RTC/RtpDictionaries/RtpCodecParameters.cpp"
    "src/RTC/RtpDictionaries/RtpEncodingParameters.cpp"
    "src/RTC/RtpDictionaries/RtpHeaderExtensionParameters.cpp"
    "src/RTC/RtpDictionaries/RtpHeaderExtensionUri.cpp"
    "src/RTC/RtpDictionaries/RtpParameters.cpp"
    "src/RTC/RtpDictionaries/RtpRtxParameters.cpp"
    "src/RTC/SctpDictionaries/SctpStreamParameters.cpp"
    "src/RTC/RTCP/Packet.cpp"
    "src/RTC/RTCP/CompoundPacket.cpp"
    "src/RTC/RTCP/SenderReport.cpp"
    "src/RTC/RTCP/ReceiverReport.cpp"
    "src/RTC/RTCP/Sdes.cpp"
    "src/RTC/RTCP/Bye.cpp"
    "src/RTC/RTCP/Feedback.cpp"
    "src/RTC/RTCP/FeedbackPs.cpp"
    "src/RTC/RTCP/FeedbackRtp.cpp"
    "src/RTC/RTCP/FeedbackRtpNack.cpp"
    "src/RTC/RTCP/FeedbackRtpTmmb.cpp"
    "src/RTC/RTCP/FeedbackRtpSrReq.cpp"
    "src/RTC/RTCP/FeedbackRtpTllei.cpp"
    "src/RTC/RTCP/FeedbackRtpEcn.cpp"
    "src/RTC/RTCP/FeedbackRtpTransport.cpp"
    "src/RTC/RTCP/FeedbackPsPli.cpp"
    "src/RTC/RTCP/FeedbackPsSli.cpp"
    "src/RTC/RTCP/FeedbackPsRpsi.cpp"
    "src/RTC/RTCP/FeedbackPsFir.cpp"
    "src/RTC/RTCP/FeedbackPsTst.cpp"
    "src/RTC/RTCP/FeedbackPsVbcm.cpp"
    "src/RTC/RTCP/FeedbackPsLei.cpp"
    "src/RTC/RTCP/FeedbackPsAfb.cpp"
    "src/RTC/RTCP/FeedbackPsRemb.cpp"
    "src/RTC/RTCP/XR.cpp"
    "src/RTC/RTCP/XrDelaySinceLastRr.cpp"
    "src/RTC/RTCP/XrReceiverReferenceTime.cpp"
)

add_executable(${PROJECT_NAME}
    ${mediasoup_worker_sources}
    ${mediasoup_worker_headers}
)

#[[
TODO:

# FIPS.
      [ 'openssl_fips != ""', {
        'defines': [ 'BUD_FIPS_ENABLED=1' ]
      }],
# Platform-specifics.

      [ 'OS == "mac" and mediasoup_asan == "true"', {
        'xcode_settings':
        {
          'OTHER_CFLAGS': [ '-fsanitize=address' ],
          'OTHER_LDFLAGS': [ '-fsanitize=address' ]
        }
      }],

      [ 'OS == "linux" and mediasoup_asan == "true"', {
        'cflags': [ '-fsanitize=address' ],
        'ldflags': [ '-fsanitize=address' ]
      }],
# Dependency-specifics.

      [ 'sctp_debug == "true"', {
        'defines': [ 'SCTP_DEBUG' ]
      }]
[ 'OS in "linux freebsd"', {
        'ldflags': [ '-Wl,--whole-archive <(libopenssl) -Wl,--no-whole-archive' ]
      }],
[ 'OS == "win"', {

        # Handle multi files with same name.
        # https://stackoverflow.com/a/22936230/2085408
        # https://docs.microsoft.com/en-us/dotnet/api/microsoft.visualstudio.vcprojectengine.vcclcompilertool.objectfile?view=visualstudiosdk-2017#Microsoft_VisualStudio_VCProjectEngine_VCCLCompilerTool_ObjectFile
        'msvs_settings': {
          'VCCLCompilerTool': { 'ObjectFile': ['$(IntDir)\%(RelativeDir)\%(Filename).obj'], },
        },

        # Output Directory setting for msvc.
        # https://github.com/nodejs/node-gyp/issues/1242#issuecomment-310921441
        'msvs_configuration_attributes': {
          'OutputDirectory': '$(SolutionDir)\\out\\$(Configuration)\\'
        }
      }],
]]

if(IS_BIG_ENDIAN)
    target_compile_definitions(${PROJECT_NAME} PRIVATE "MS_BIG_ENDIAN")
else()
    target_compile_definitions(${PROJECT_NAME} PRIVATE "MS_LITTLE_ENDIAN")
endif()

IF(CMAKE_SYSTEM_NAME MATCHES "Linux")
    target_compile_definitions(${PROJECT_NAME} 
        PRIVATE 
        _POSIX_C_SOURCE=200112
        _GNU_SOURCE
    )
ELSEIF(CMAKE_SYSTEM_NAME MATCHES "FreeBSD")
    set(CMAKE_SHARED_LINKER_FLAGS "-Wl,--export-dynamic")
ELSEIF(CMAKE_SYSTEM_NAME MATCHES "Windows")
    target_link_libraries(${PROJECT_NAME}
        wingetopt
    )
    target_include_directories(${PROJECT_NAME}
    PRIVATE
    deps/getopt/getopt/src
)
ELSE()

ENDIF(CMAKE_SYSTEM_NAME MATCHES "Linux")

IF(NOT WIN32)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11 -Wall -Wextra -Wno-unused-parameter -Wno-implicit-fallthrough")
ENDIF()

IF(APPLE)
    set_property(TARGET ${PROJECT_NAME}
        PROPERTY XCODE_ATTRIBUTE_WARNING_CFLAGS
        -Wall
        -Wextra
        -Wno-unused-parameter
        PROPERTY XCODE_ATTRIBUTE_OTHER_CPLUSPLUSFLAGS
        -std=c++11
    )
ENDIF()

target_include_directories(${PROJECT_NAME}
    PRIVATE
    include
    deps/json/single_include/nlohmann
    deps/usrsctp/usrsctp/usrsctplib
)

target_link_libraries(${PROJECT_NAME}
    netstring
    uv
    ssl
    # crypto
    srtp2
    libwebrtc
)

IF(WIN32)
    target_link_libraries(${PROJECT_NAME}
        usrsctp
    )
ELSE()
    target_link_libraries(${PROJECT_NAME}
        usrsctp
    )
ENDIF()

install(TARGETS ${PROJECT_NAME} DESTINATION lib/$<CONFIG>)



