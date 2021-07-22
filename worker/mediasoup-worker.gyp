{
  'target_defaults': {
    'type': 'executable',
    'dependencies':
    [
      'deps/netstring/netstring.gyp:netstring',
      'deps/libuv/uv.gyp:libuv',
      'deps/openssl/openssl.gyp:openssl',
      'deps/libsrtp/libsrtp.gyp:libsrtp',
      'deps/usrsctp/usrsctp.gyp:usrsctp',
      'deps/libwebrtc/libwebrtc.gyp:libwebrtc',
      'deps/libwebrtc/deps/abseil-cpp/abseil-cpp.gyp:abseil'
    ],
    # TODO: SCTP_DEBUG must be dynamic based on a condition variable in common.gyp.
    # 'defines': [ 'SCTP_DEBUG' ],
    'sources':
    [
      # C++ source files.
      'src/lib.cpp',
      'src/DepLibSRTP.cpp',
      'src/DepLibUV.cpp',
      'src/DepLibWebRTC.cpp',
      'src/DepOpenSSL.cpp',
      'src/DepUsrSCTP.cpp',
      'src/Logger.cpp',
      'src/MediaSoupErrors.cpp',
      'src/Settings.cpp',
      'src/Worker.cpp',
      'src/Utils/Crypto.cpp',
      'src/Utils/File.cpp',
      'src/Utils/IP.cpp',
      'src/Utils/String.cpp',
      'src/handles/SignalsHandler.cpp',
      'src/handles/TcpConnectionHandler.cpp',
      'src/handles/TcpServerHandler.cpp',
      'src/handles/Timer.cpp',
      'src/handles/UdpSocketHandler.cpp',
      'src/handles/UnixStreamSocket.cpp',
      'src/Channel/ChannelNotifier.cpp',
      'src/Channel/ChannelRequest.cpp',
      'src/Channel/ChannelSocket.cpp',
      'src/PayloadChannel/Notification.cpp',
      'src/PayloadChannel/PayloadChannelNotifier.cpp',
      'src/PayloadChannel/PayloadChannelRequest.cpp',
      'src/PayloadChannel/PayloadChannelSocket.cpp',
      'src/RTC/ActiveSpeakerObserver.cpp',
      'src/RTC/AudioLevelObserver.cpp',
      'src/RTC/Consumer.cpp',
      'src/RTC/DataConsumer.cpp',
      'src/RTC/DataProducer.cpp',
      'src/RTC/DirectTransport.cpp',
      'src/RTC/DtlsTransport.cpp',
      'src/RTC/IceCandidate.cpp',
      'src/RTC/IceServer.cpp',
      'src/RTC/KeyFrameRequestManager.cpp',
      'src/RTC/NackGenerator.cpp',
      'src/RTC/PipeConsumer.cpp',
      'src/RTC/PipeTransport.cpp',
      'src/RTC/PlainTransport.cpp',
      'src/RTC/PortManager.cpp',
      'src/RTC/Producer.cpp',
      'src/RTC/RateCalculator.cpp',
      'src/RTC/Router.cpp',
      'src/RTC/RtpListener.cpp',
      'src/RTC/RtpObserver.cpp',
      'src/RTC/RtpPacket.cpp',
      'src/RTC/RtpProbationGenerator.cpp',
      'src/RTC/RtpStream.cpp',
      'src/RTC/RtpStreamRecv.cpp',
      'src/RTC/RtpStreamSend.cpp',
      'src/RTC/RtxStream.cpp',
      'src/RTC/SctpAssociation.cpp',
      'src/RTC/SctpListener.cpp',
      'src/RTC/SenderBandwidthEstimator.cpp',
      'src/RTC/SeqManager.cpp',
      'src/RTC/SimpleConsumer.cpp',
      'src/RTC/SimulcastConsumer.cpp',
      'src/RTC/SrtpSession.cpp',
      'src/RTC/StunPacket.cpp',
      'src/RTC/SvcConsumer.cpp',
      'src/RTC/TcpConnection.cpp',
      'src/RTC/TcpServer.cpp',
      'src/RTC/Transport.cpp',
      'src/RTC/TransportCongestionControlClient.cpp',
      'src/RTC/TransportCongestionControlServer.cpp',
      'src/RTC/TransportTuple.cpp',
      'src/RTC/TrendCalculator.cpp',
      'src/RTC/UdpSocket.cpp',
      'src/RTC/WebRtcTransport.cpp',
      'src/RTC/Codecs/H264.cpp',
      'src/RTC/Codecs/VP8.cpp',
      'src/RTC/Codecs/VP9.cpp',
      'src/RTC/RtpDictionaries/Media.cpp',
      'src/RTC/RtpDictionaries/Parameters.cpp',
      'src/RTC/RtpDictionaries/RtcpFeedback.cpp',
      'src/RTC/RtpDictionaries/RtcpParameters.cpp',
      'src/RTC/RtpDictionaries/RtpCodecMimeType.cpp',
      'src/RTC/RtpDictionaries/RtpCodecParameters.cpp',
      'src/RTC/RtpDictionaries/RtpEncodingParameters.cpp',
      'src/RTC/RtpDictionaries/RtpHeaderExtensionParameters.cpp',
      'src/RTC/RtpDictionaries/RtpHeaderExtensionUri.cpp',
      'src/RTC/RtpDictionaries/RtpParameters.cpp',
      'src/RTC/RtpDictionaries/RtpRtxParameters.cpp',
      'src/RTC/SctpDictionaries/SctpStreamParameters.cpp',
      'src/RTC/RTCP/Packet.cpp',
      'src/RTC/RTCP/CompoundPacket.cpp',
      'src/RTC/RTCP/SenderReport.cpp',
      'src/RTC/RTCP/ReceiverReport.cpp',
      'src/RTC/RTCP/Sdes.cpp',
      'src/RTC/RTCP/Bye.cpp',
      'src/RTC/RTCP/Feedback.cpp',
      'src/RTC/RTCP/FeedbackPs.cpp',
      'src/RTC/RTCP/FeedbackRtp.cpp',
      'src/RTC/RTCP/FeedbackRtpNack.cpp',
      'src/RTC/RTCP/FeedbackRtpTmmb.cpp',
      'src/RTC/RTCP/FeedbackRtpSrReq.cpp',
      'src/RTC/RTCP/FeedbackRtpTllei.cpp',
      'src/RTC/RTCP/FeedbackRtpEcn.cpp',
      'src/RTC/RTCP/FeedbackRtpTransport.cpp',
      'src/RTC/RTCP/FeedbackPsPli.cpp',
      'src/RTC/RTCP/FeedbackPsSli.cpp',
      'src/RTC/RTCP/FeedbackPsRpsi.cpp',
      'src/RTC/RTCP/FeedbackPsFir.cpp',
      'src/RTC/RTCP/FeedbackPsTst.cpp',
      'src/RTC/RTCP/FeedbackPsVbcm.cpp',
      'src/RTC/RTCP/FeedbackPsLei.cpp',
      'src/RTC/RTCP/FeedbackPsAfb.cpp',
      'src/RTC/RTCP/FeedbackPsRemb.cpp',
      'src/RTC/RTCP/XR.cpp',
      'src/RTC/RTCP/XrDelaySinceLastRr.cpp',
      'src/RTC/RTCP/XrReceiverReferenceTime.cpp',
      # C++ include files.
      'include/lib.hpp',
      'include/DepLibSRTP.hpp',
      'include/DepLibUV.hpp',
      'include/DepLibWebRTC.hpp',
      'include/DepOpenSSL.hpp',
      'include/DepUsrSCTP.hpp',
      'include/LogLevel.hpp',
      'include/Logger.hpp',
      'include/MediaSoupErrors.hpp',
      'include/Settings.hpp',
      'include/Utils.hpp',
      'include/Worker.hpp',
      'include/common.hpp',
      'include/handles/SignalsHandler.hpp',
      'include/handles/TcpConnectionHandler.hpp',
      'include/handles/TcpServerHandler.hpp',
      'include/handles/Timer.hpp',
      'include/handles/UdpSocketHandler.hpp',
      'include/handles/UnixStreamSocket.hpp',
      'include/Channel/ChannelNotifier.hpp',
      'include/Channel/ChannelRequest.hpp',
      'include/Channel/ChannelSocket.hpp',
      'include/PayloadChannel/Notification.hpp',
      'include/PayloadChannel/PayloadChannelNotifier.hpp',
      'include/PayloadChannel/PayloadChannelRequest.hpp',
      'include/PayloadChannel/PayloadChannelSocket.hpp',
      'include/RTC/BweType.hpp',
      'include/RTC/ActiveSpeakerObserver.hpp',
      'include/RTC/AudioLevelObserver.hpp',
      'include/RTC/Consumer.hpp',
      'include/RTC/DataConsumer.hpp',
      'include/RTC/DirectTransport.hpp',
      'include/RTC/DataProducer.hpp',
      'include/RTC/DtlsTransport.hpp',
      'include/RTC/IceCandidate.hpp',
      'include/RTC/IceServer.hpp',
      'include/RTC/KeyFrameRequestManager.hpp',
      'include/RTC/NackGenerator.hpp',
      'include/RTC/Parameters.hpp',
      'include/RTC/PipeConsumer.hpp',
      'include/RTC/PipeTransport.hpp',
      'include/RTC/PlainTransport.hpp',
      'include/RTC/PortManager.hpp',
      'include/RTC/Producer.hpp',
      'include/RTC/RateCalculator.hpp',
      'include/RTC/Router.hpp',
      'include/RTC/RtpDictionaries.hpp',
      'include/RTC/RtpHeaderExtensionIds.hpp',
      'include/RTC/RtpListener.hpp',
      'include/RTC/RtpObserver.hpp',
      'include/RTC/RtpPacket.hpp',
      'include/RTC/RtpProbationGenerator.hpp',
      'include/RTC/RtpStream.hpp',
      'include/RTC/RtpStreamRecv.hpp',
      'include/RTC/RtpStreamSend.hpp',
      'include/RTC/RtxStream.hpp',
      'include/RTC/SctpAssociation.hpp',
      'include/RTC/SctpDictionaries.hpp',
      'include/RTC/SctpListener.hpp',
      'include/RTC/SenderBandwidthEstimator.hpp',
      'include/RTC/SeqManager.hpp',
      'include/RTC/SimpleConsumer.hpp',
      'include/RTC/SimulcastConsumer.hpp',
      'include/RTC/SrtpSession.hpp',
      'include/RTC/StunPacket.hpp',
      'include/RTC/SvcConsumer.hpp',
      'include/RTC/TcpConnection.hpp',
      'include/RTC/TcpServer.hpp',
      'include/RTC/Transport.hpp',
      'include/RTC/TransportCongestionControlClient.hpp',
      'include/RTC/TransportCongestionControlServer.hpp',
      'include/RTC/TransportTuple.hpp',
      'include/RTC/TrendCalculator.hpp',
      'include/RTC/UdpSocket.hpp',
      'include/RTC/WebRtcTransport.hpp',
      'include/RTC/Codecs/Tools.hpp',
      'include/RTC/Codecs/PayloadDescriptorHandler.hpp',
      'include/RTC/Codecs/H264.hpp',
      'include/RTC/Codecs/VP8.hpp',
      'include/RTC/Codecs/VP9.hpp',
      'include/RTC/RTCP/Packet.hpp',
      'include/RTC/RTCP/CompoundPacket.hpp',
      'include/RTC/RTCP/SenderReport.hpp',
      'include/RTC/RTCP/ReceiverReport.hpp',
      'include/RTC/RTCP/Sdes.hpp',
      'include/RTC/RTCP/Bye.hpp',
      'include/RTC/RTCP/Feedback.hpp',
      'include/RTC/RTCP/FeedbackItem.hpp',
      'include/RTC/RTCP/FeedbackPs.hpp',
      'include/RTC/RTCP/FeedbackRtp.hpp',
      'include/RTC/RTCP/FeedbackRtpNack.hpp',
      'include/RTC/RTCP/FeedbackRtpTmmb.hpp',
      'include/RTC/RTCP/FeedbackRtpSrReq.hpp',
      'include/RTC/RTCP/FeedbackRtpTllei.hpp',
      'include/RTC/RTCP/FeedbackRtpEcn.hpp',
      'include/RTC/RTCP/FeedbackRtpTransport.hpp',
      'include/RTC/RTCP/FeedbackPsPli.hpp',
      'include/RTC/RTCP/FeedbackPsSli.hpp',
      'include/RTC/RTCP/FeedbackPsRpsi.hpp',
      'include/RTC/RTCP/FeedbackPsFir.hpp',
      'include/RTC/RTCP/FeedbackPsTst.hpp',
      'include/RTC/RTCP/FeedbackPsVbcm.hpp',
      'include/RTC/RTCP/FeedbackPsLei.hpp',
      'include/RTC/RTCP/FeedbackPsAfb.hpp',
      'include/RTC/RTCP/FeedbackPsRemb.hpp',
      'include/RTC/RTCP/XR.hpp',
      'include/RTC/RTCP/XrDelaySinceLastRr.hpp',
      'include/RTC/RTCP/XrReceiverReferenceTime.hpp'
    ],
    'include_dirs':
    [
      'include',
      'deps/json/single_include'
    ],
    'conditions':
    [
      # FIPS.
      [ 'openssl_fips != ""', {
        'defines': [ 'BUD_FIPS_ENABLED=1' ]
      }],

      # Endianness.
      [ 'node_byteorder == "big"', {
          # Define Big Endian.
          'defines': [ 'MS_BIG_ENDIAN' ]
        }, {
          # Define Little Endian.
          'defines': [ 'MS_LITTLE_ENDIAN' ]
      }],

      # Platform-specifics.

      [ 'OS == "mac" and mediasoup_asan == "true"', {
        'xcode_settings':
        {
          'OTHER_CFLAGS': [ '-fsanitize=address' ],
          'OTHER_LDFLAGS': [ '-fsanitize=address' ]
        }
      }],

      [ 'OS == "linux"', {
        'defines':
        [
          '_POSIX_C_SOURCE=200112',
          '_GNU_SOURCE'
        ]
      }],

      [ 'OS == "linux" and mediasoup_asan == "true"', {
        'cflags': [ '-fsanitize=address' ],
        'ldflags': [ '-fsanitize=address' ]
      }],

      [ 'OS in "linux freebsd"', {
        'ldflags': [ '-Wl,--whole-archive <(libopenssl) -Wl,--no-whole-archive' ]
      }],

      [ 'OS in "freebsd"', {
        'ldflags': [ '-Wl,--export-dynamic' ]
      }],

      [ 'OS == "win"', {
        'dependencies': [ 'deps/getopt/getopt.gyp:getopt' ],

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

      [ 'OS != "win"', {
        'cflags': [ '-std=c++11', '-Wall', '-Wextra', '-Wno-unused-parameter', '-Wno-implicit-fallthrough' ]
      }],

      [ 'OS == "mac"', {
        'xcode_settings':
        {
          'WARNING_CFLAGS': [ '-Wall', '-Wextra', '-Wno-unused-parameter' ],
          'OTHER_CPLUSPLUSFLAGS' : [ '-std=c++11' ]
        }
      }],

      # Dependency-specifics.

      [ 'sctp_debug == "true"', {
        'defines': [ 'SCTP_DEBUG' ]
      }]
    ]
  },
  'targets':
  [
    {
      'target_name': 'mediasoup-worker',
      'sources':
      [
        # C++ source files.
        'src/main.cpp'
      ],
	  'defines': [ 'MS_EXECUTABLE' ]
    },
    {
      'target_name': 'libmediasoup-worker',
      'type': 'static_library'
    },
    {
      'target_name': 'mediasoup-worker-test',
      'defines': [ 'MS_LOG_STD', 'MS_TEST' ],
      'sources':
      [
        # C++ source files.
        'test/src/tests.cpp',
        'test/src/RTC/TestKeyFrameRequestManager.cpp',
        'test/src/RTC/TestNackGenerator.cpp',
        'test/src/RTC/TestRateCalculator.cpp',
        'test/src/RTC/TestRtpPacket.cpp',
        'test/src/RTC/TestRtpStreamSend.cpp',
        'test/src/RTC/TestRtpStreamRecv.cpp',
        'test/src/RTC/TestSeqManager.cpp',
        'test/src/RTC/TestTrendCalculator.cpp',
        'test/src/RTC/TestRtpEncodingParameters.cpp',
        'test/src/RTC/Codecs/TestVP8.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsAfb.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsFir.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsLei.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsPli.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsRemb.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsRpsi.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsSli.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsTst.cpp',
        'test/src/RTC/RTCP/TestFeedbackPsVbcm.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpEcn.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpNack.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpSrReq.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpTllei.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpTmmb.cpp',
        'test/src/RTC/RTCP/TestFeedbackRtpTransport.cpp',
        'test/src/RTC/RTCP/TestBye.cpp',
        'test/src/RTC/RTCP/TestReceiverReport.cpp',
        'test/src/RTC/RTCP/TestSdes.cpp',
        'test/src/RTC/RTCP/TestSenderReport.cpp',
        'test/src/RTC/RTCP/TestPacket.cpp',
        'test/src/RTC/RTCP/TestXr.cpp',
        'test/src/Utils/TestBits.cpp',
        'test/src/Utils/TestIP.cpp',
        'test/src/Utils/TestJson.cpp',
        'test/src/Utils/TestString.cpp',
        'test/src/Utils/TestTime.cpp',
        # C++ include files.
        'test/include/helpers.hpp'
      ],
      'include_dirs':
      [
        'test/include',
        'deps/catch2/single_include'
      ],
      'xcode_settings':
      {
        'OTHER_CPLUSPLUSFLAGS':
        [
          '--coverage'
        ],
        'OTHER_LDFLAGS': [
          '--coverage'
        ]
      }
    },
    {
      'target_name': 'mediasoup-worker-fuzzer',
      'defines': [ 'DEBUG', 'MS_LOG_STD', 'MS_TEST' ],
      'sources':
      [
        # C++ source files.
        'fuzzer/src/fuzzer.cpp',
        'fuzzer/src/FuzzerUtils.cpp',
        'fuzzer/src/RTC/FuzzerStunPacket.cpp',
        'fuzzer/src/RTC/FuzzerRtpPacket.cpp',
        'fuzzer/src/RTC/FuzzerTrendCalculator.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerBye.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPs.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsAfb.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsFir.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsLei.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsPli.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsRemb.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsRpsi.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsSli.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsTst.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackPsVbcm.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtp.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpEcn.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpNack.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpSrReq.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpTllei.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpTmmb.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerFeedbackRtpTransport.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerPacket.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerReceiverReport.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerSdes.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerSenderReport.cpp',
        'fuzzer/src/RTC/RTCP/FuzzerXr.cpp',
        # C++ include files.
        'fuzzer/include/FuzzerUtils.hpp',
        'fuzzer/include/RTC/FuzzerRtpPacket.hpp',
        'fuzzer/include/RTC/FuzzerStunPacket.hpp',
        'fuzzer/include/RTC/FuzzerTrendCalculator.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerBye.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPs.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsAfb.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsFir.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsLei.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsPli.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsRemb.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsRpsi.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsSli.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsTst.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackPsVbcm.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtp.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpEcn.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpNack.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpSrReq.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpTllei.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpTmmb.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerFeedbackRtpTransport.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerPacket.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerReceiverReport.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerSdes.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerSenderReport.hpp',
        'fuzzer/include/RTC/RTCP/FuzzerXr.hpp',
      ],
      'include_dirs':
      [
        'fuzzer/include'
      ],

      'conditions':
      [
        [ 'OS == "linux"', {
          'cflags': [ '-g', '-O0', '-fsanitize=address,fuzzer' ],
          'ldflags': [ '-fsanitize=address,fuzzer' ]
        }]
      ]
    }
  ]
}
