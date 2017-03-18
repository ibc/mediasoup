/*
 *  Copyright (c) 2013 The WebRTC project authors. All Rights Reserved.
 *
 *  Use of this source code is governed by a BSD-style license
 *  that can be found in the LICENSE file in the root of the source
 *  tree. An additional intellectual property rights grant can be found
 *  in the file PATENTS.  All contributing project authors may
 *  be found in the AUTHORS file in the root of the source tree.
 */
#ifndef MS_RTC_REMOTE_BITRATE_ESTIMATOR_OVERUSE_ESTIMATOR_HPP
#define MS_RTC_REMOTE_BITRATE_ESTIMATOR_OVERUSE_ESTIMATOR_HPP

#include "common.hpp"
#include "RTC/RemoteBitrateEstimator/BandwidthUsage.hpp"
#include <deque>

namespace RTC {

// (jmillan) borrowed from webrtc/common_types.h
//
// Bandwidth over-use detector options.  These are used to drive
// experimentation with bandwidth estimation parameters.
// See modules/remote_bitrate_estimator/overuse_detector.h
// TODO(terelius): This is only used in overuse_estimator.cc, and only in the
// default constructed state. Can we move the relevant variables into that
// class and delete this? See also disabled warning at line 27
struct OverUseDetectorOptions {
  OverUseDetectorOptions()
      : initialSlope(8.0 / 512.0),
        initialOffset(0),
        initial_e(),
        initialProcessNoise(),
        initialAvgNoise(0.0),
        initialVarNoise(50) {
    initial_e[0][0] = 100;
    initial_e[1][1] = 1e-1;
    initial_e[0][1] = initial_e[1][0] = 0;
    initialProcessNoise[0] = 1e-13;
    initialProcessNoise[1] = 1e-3;
  }
  double initialSlope;
  double initialOffset;
  double initial_e[2][2];
  double initialProcessNoise[2];
  double initialAvgNoise;
  double initialVarNoise;
};

class OveruseEstimator {
 public:
  explicit OveruseEstimator(const OverUseDetectorOptions& options);
  ~OveruseEstimator();

  // Update the estimator with a new sample. The deltas should represent deltas
  // between timestamp groups as defined by the InterArrival class.
  // |current_hypothesis| should be the hypothesis of the over-use detector at
  // this time.
  void Update(int64_t tDelta,
              double tsDelta,
              int sizeDelta,
              BandwidthUsage currentHypothesis,
              int64_t nowMs);

  // Returns the estimated noise/jitter variance in ms^2.
  double GetVarNoise() const {
    return this->varNoise;
  }

  // Returns the estimated inter-arrival time delta offset in ms.
  double GetOffset() const {
    return this->offset;
  }

  // Returns the number of deltas which the current over-use estimator state is
  // based on.
  unsigned int GetNumOfDeltas() const {
    return this->numOfDeltas;
  }

 private:
  double UpdateMinFramePeriod(double tsDelta);
  void UpdateNoiseEstimate(double residual, double tsDelta, bool stableState);

  // Must be first member variable. Cannot be const because we need to be
  // copyable.
  OverUseDetectorOptions options;
  uint16_t numOfDeltas;
  double slope;
  double offset;
  double prevOffset;
  double E[2][2];
  double processNoise[2];
  double avgNoise;
  double varNoise;
  std::deque<double> tsDeltaHist;

};
}  // namespace RTC

#endif  // MS_RTC_REMOTE_BITRATE_ESTIMATOR_OVERUSE_ESTIMATOR_HPP
