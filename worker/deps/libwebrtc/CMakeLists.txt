cmake_minimum_required(VERSION 3.16)

set(PROJECT_NAME "libwebrtc")
project(${PROJECT_NAME})

include (TestBigEndian)
TEST_BIG_ENDIAN(IS_BIG_ENDIAN)

IF(NOT WIN32)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=c++11")
ENDIF()

set(BUILD_TESTING OFF CACHE INTERNAL "")
add_subdirectory(deps/abseil-cpp/abseil-cpp)
unset(BUILD_TESTING)

set(libwebrtc_headers
    libwebrtc/system_wrappers/source/field_trial.h
    libwebrtc/rtc_base/rate_statistics.h
    libwebrtc/rtc_base/experiments/field_trial_parser.h
    libwebrtc/rtc_base/experiments/field_trial_units.h
    libwebrtc/rtc_base/experiments/alr_experiment.h
    libwebrtc/rtc_base/experiments/rate_control_settings.h
    libwebrtc/rtc_base/network/sent_packet.h
    libwebrtc/rtc_base/units/unit_base.h
    libwebrtc/rtc_base/constructor_magic.h
    libwebrtc/rtc_base/numerics/safe_minmax.h
    libwebrtc/rtc_base/numerics/safe_conversions.h
    libwebrtc/rtc_base/numerics/safe_conversions_impl.h
    libwebrtc/rtc_base/numerics/percentile_filter.h
    libwebrtc/rtc_base/numerics/safe_compare.h
    libwebrtc/rtc_base/system/unused.h
    libwebrtc/rtc_base/type_traits.h
    libwebrtc/call/rtp_transport_controller_send.h
    libwebrtc/call/rtp_transport_controller_send_interface.h
    libwebrtc/api/transport/webrtc_key_value_config.h
    libwebrtc/api/transport/network_types.h
    libwebrtc/api/transport/bitrate_settings.h
    libwebrtc/api/transport/network_control.h
    libwebrtc/api/transport/field_trial_based_config.h
    libwebrtc/api/transport/goog_cc_factory.h
    libwebrtc/api/bitrate_constraints.h
    libwebrtc/api/units/frequency.h
    libwebrtc/api/units/data_size.h
    libwebrtc/api/units/time_delta.h
    libwebrtc/api/units/data_rate.h
    libwebrtc/api/units/timestamp.h
    libwebrtc/api/network_state_predictor.h
    libwebrtc/modules/include/module_common_types_public.h
    libwebrtc/modules/pacing/interval_budget.h
    libwebrtc/modules/pacing/paced_sender.h
    libwebrtc/modules/pacing/packet_router.h
    libwebrtc/modules/pacing/bitrate_prober.h
    libwebrtc/modules/remote_bitrate_estimator/inter_arrival.h
    libwebrtc/modules/remote_bitrate_estimator/overuse_detector.h
    libwebrtc/modules/remote_bitrate_estimator/overuse_estimator.h
    libwebrtc/modules/remote_bitrate_estimator/include/bwe_defines.h
    libwebrtc/modules/remote_bitrate_estimator/aimd_rate_control.h
    libwebrtc/modules/remote_bitrate_estimator/remote_bitrate_estimator_abs_send_time.h
    libwebrtc/modules/remote_bitrate_estimator/include/remote_bitrate_estimator.h
    libwebrtc/modules/rtp_rtcp/include/rtp_rtcp_defines.h
    libwebrtc/modules/rtp_rtcp/source/rtp_packet/transport_feedback.h
    libwebrtc/modules/bitrate_controller/loss_based_bandwidth_estimation.h
    libwebrtc/modules/bitrate_controller/send_side_bandwidth_estimation.h
    libwebrtc/modules/congestion_controller/goog_cc/bitrate_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/link_capacity_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/median_slope_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/probe_controller.h
    libwebrtc/modules/congestion_controller/goog_cc/trendline_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/goog_cc_network_control.h
    libwebrtc/modules/congestion_controller/goog_cc/delay_increase_detector_interface.h
    libwebrtc/modules/congestion_controller/goog_cc/acknowledged_bitrate_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/congestion_window_pushback_controller.h
    libwebrtc/modules/congestion_controller/goog_cc/delay_based_bwe.h
    libwebrtc/modules/congestion_controller/goog_cc/probe_bitrate_estimator.h
    libwebrtc/modules/congestion_controller/goog_cc/alr_detector.h
    libwebrtc/modules/congestion_controller/rtp/send_time_history.h
    libwebrtc/modules/congestion_controller/rtp/transport_feedback_adapter.h
    libwebrtc/modules/congestion_controller/rtp/control_handler.h
    libwebrtc/mediasoup_helpers.h
)
set(libwebrtc_sources
    libwebrtc/system_wrappers/source/field_trial.cc
    libwebrtc/rtc_base/rate_statistics.cc
    libwebrtc/rtc_base/experiments/field_trial_parser.cc
    libwebrtc/rtc_base/experiments/alr_experiment.cc
    libwebrtc/rtc_base/experiments/field_trial_units.cc
    libwebrtc/rtc_base/experiments/rate_control_settings.cc
    libwebrtc/rtc_base/network/sent_packet.cc
    libwebrtc/call/rtp_transport_controller_send.cc
    libwebrtc/api/transport/bitrate_settings.cc
    libwebrtc/api/transport/field_trial_based_config.cc
    libwebrtc/api/transport/network_types.cc
    libwebrtc/api/transport/goog_cc_factory.cc
    libwebrtc/api/units/timestamp.cc
    libwebrtc/api/units/time_delta.cc
    libwebrtc/api/units/data_rate.cc
    libwebrtc/api/units/data_size.cc
    libwebrtc/api/units/frequency.cc
    libwebrtc/api/network_state_predictor.cc
    libwebrtc/modules/pacing/interval_budget.cc
    libwebrtc/modules/pacing/bitrate_prober.cc
    libwebrtc/modules/pacing/paced_sender.cc
    libwebrtc/modules/remote_bitrate_estimator/overuse_detector.cc
    libwebrtc/modules/remote_bitrate_estimator/overuse_estimator.cc
    libwebrtc/modules/remote_bitrate_estimator/aimd_rate_control.cc
    libwebrtc/modules/remote_bitrate_estimator/inter_arrival.cc
    libwebrtc/modules/remote_bitrate_estimator/bwe_defines.cc
    libwebrtc/modules/remote_bitrate_estimator/remote_bitrate_estimator_abs_send_time.cc
    libwebrtc/modules/rtp_rtcp/include/rtp_rtcp_defines.cc
    libwebrtc/modules/bitrate_controller/send_side_bandwidth_estimation.cc
    libwebrtc/modules/bitrate_controller/loss_based_bandwidth_estimation.cc
    libwebrtc/modules/congestion_controller/goog_cc/goog_cc_network_control.cc
    libwebrtc/modules/congestion_controller/goog_cc/probe_bitrate_estimator.cc
    libwebrtc/modules/congestion_controller/goog_cc/congestion_window_pushback_controller.cc
    libwebrtc/modules/congestion_controller/goog_cc/link_capacity_estimator.cc
    libwebrtc/modules/congestion_controller/goog_cc/alr_detector.cc
    libwebrtc/modules/congestion_controller/goog_cc/probe_controller.cc
    libwebrtc/modules/congestion_controller/goog_cc/median_slope_estimator.cc
    libwebrtc/modules/congestion_controller/goog_cc/bitrate_estimator.cc
    libwebrtc/modules/congestion_controller/goog_cc/trendline_estimator.cc
    libwebrtc/modules/congestion_controller/goog_cc/delay_based_bwe.cc
    libwebrtc/modules/congestion_controller/goog_cc/acknowledged_bitrate_estimator.cc
    libwebrtc/modules/congestion_controller/rtp/send_time_history.cc
    libwebrtc/modules/congestion_controller/rtp/transport_feedback_adapter.cc
    libwebrtc/modules/congestion_controller/rtp/control_handler.cc
)

# file(GLOB_RECURSE libwebrtc_headers libwebrtc/*.h??)
# file(GLOB_RECURSE libwebrtc_sources libwebrtc/*.cc??)

add_library(${PROJECT_NAME}
  STATIC
  ${libwebrtc_sources}
  ${libwebrtc_headers}
)

IF(APPLE)
    set_property(TARGET ${PROJECT_NAME}
        PROPERTY XCODE_ATTRIBUTE_OTHER_CPLUSPLUSFLAGS
        -std=c++11
    )
ENDIF()

if(IS_BIG_ENDIAN)
    target_compile_definitions(${PROJECT_NAME} PRIVATE "MS_BIG_ENDIAN")
else()
    target_compile_definitions(${PROJECT_NAME} PRIVATE "MS_LITTLE_ENDIAN")
endif()

target_link_libraries(${PROJECT_NAME}
    uv
    ssl
    absl_internal_throw_delegate
    absl_strings
    absl_optional
)

target_include_directories(${PROJECT_NAME}
    PRIVATE
    libwebrtc
    ../../include
    ../json/single_include/nlohmann
    ${CMAKE_CURRENT_SOURCE_DIR}/libwebrtc
)

target_include_directories(${PROJECT_NAME}
    INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/libwebrtc
    ${CMAKE_CURRENT_SOURCE_DIR}/deps/abseil-cpp
)

install(TARGETS ${PROJECT_NAME} DESTINATION lib)
install(FILES ${libwebrtc_headers} DESTINATION include)
