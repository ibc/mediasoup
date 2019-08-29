/*
 *  Copyright (c) 2018 The WebRTC project authors. All Rights Reserved.
 *
 *  Use of this source code is governed by a BSD-style license
 *  that can be found in the LICENSE file in the root of the source
 *  tree. An additional intellectual property rights grant can be found
 *  in the file PATENTS.  All contributing project authors may
 *  be found in the AUTHORS file in the root of the source tree.
 */

#include "RTC/SendTransportController/data_size.h"
#include <sstream>

namespace webrtc {

std::string ToString(DataSize value) {
  std::ostringstream sb;
  if (value.IsPlusInfinity()) {
    sb << "+inf bytes";
  } else if (value.IsMinusInfinity()) {
    sb << "-inf bytes";
  } else {
    sb << value.bytes() << " bytes";
  }
  return sb.str();
}
}  // namespace webrtc