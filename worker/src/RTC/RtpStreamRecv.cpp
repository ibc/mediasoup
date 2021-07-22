#define MS_CLASS "RTC::RtpStreamRecv"
// #define MS_LOG_DEV_LEVEL 3

#include "RTC/RtpStreamRecv.hpp"
#include "Logger.hpp"
#include "Settings.hpp"
#include "Utils.hpp"
#include "RTC/Codecs/Tools.hpp"

namespace RTC
{
	/* Static. */

	static constexpr uint64_t InactivityCheckInterval{ 1500u };        // In ms.
	static constexpr uint64_t InactivityCheckIntervalWithDtx{ 5000u }; // In ms.

	/* TransmissionCounter methods. */

	RtpStreamRecv::TransmissionCounter::TransmissionCounter(
	  uint8_t spatialLayers, uint8_t temporalLayers, size_t windowSize)
	{
		MS_TRACE();

		// Reserve vectors capacity.
		this->spatialLayerCounters = std::vector<std::vector<RTC::RtpDataCounter>>(spatialLayers);

		for (auto& spatialLayerCounter : this->spatialLayerCounters)
		{
			for (uint8_t tIdx{ 0u }; tIdx < temporalLayers; ++tIdx)
			{
				spatialLayerCounter.emplace_back(RTC::RtpDataCounter(windowSize));
			}
		}
	}

	void RtpStreamRecv::TransmissionCounter::Update(RTC::RtpPacket* packet)
	{
		MS_TRACE();

		auto spatialLayer  = packet->GetSpatialLayer();
		auto temporalLayer = packet->GetTemporalLayer();

		// Sanity check. Do not allow spatial layers higher than defined.
		if (spatialLayer > this->spatialLayerCounters.size() - 1)
			spatialLayer = this->spatialLayerCounters.size() - 1;

		// Sanity check. Do not allow temporal layers higher than defined.
		if (temporalLayer > this->spatialLayerCounters[0].size() - 1)
			temporalLayer = this->spatialLayerCounters[0].size() - 1;

		auto& counter = this->spatialLayerCounters[spatialLayer][temporalLayer];

		counter.Update(packet);
	}

	uint32_t RtpStreamRecv::TransmissionCounter::GetBitrate(uint64_t nowMs)
	{
		MS_TRACE();

		uint32_t rate{ 0u };

		for (auto& spatialLayerCounter : this->spatialLayerCounters)
		{
			for (auto& temporalLayerCounter : spatialLayerCounter)
			{
				rate += temporalLayerCounter.GetBitrate(nowMs);
			}
		}

		return rate;
	}

	uint32_t RtpStreamRecv::TransmissionCounter::GetBitrate(
	  uint64_t nowMs, uint8_t spatialLayer, uint8_t temporalLayer)
	{
		MS_TRACE();

		MS_ASSERT(spatialLayer < this->spatialLayerCounters.size(), "spatialLayer too high");
		MS_ASSERT(
		  temporalLayer < this->spatialLayerCounters[spatialLayer].size(), "temporalLayer too high");

		// Return 0 if specified layers are not being received.
		auto& counter = this->spatialLayerCounters[spatialLayer][temporalLayer];

		if (counter.GetBitrate(nowMs) == 0)
			return 0u;

		uint32_t rate{ 0u };

		// Iterate all temporal layers of spatial layers previous to the given one.
		for (uint8_t sIdx{ 0u }; sIdx < spatialLayer; ++sIdx)
		{
			for (uint8_t tIdx{ 0u }; tIdx < this->spatialLayerCounters[sIdx].size(); ++tIdx)
			{
				auto& temporalLayerCounter = this->spatialLayerCounters[sIdx][tIdx];

				rate += temporalLayerCounter.GetBitrate(nowMs);
			}
		}

		// Add the given spatial layer with up to the given temporal layer.
		for (uint8_t tIdx{ 0u }; tIdx <= temporalLayer; ++tIdx)
		{
			auto& temporalLayerCounter = this->spatialLayerCounters[spatialLayer][tIdx];

			rate += temporalLayerCounter.GetBitrate(nowMs);
		}

		return rate;
	}

	uint32_t RtpStreamRecv::TransmissionCounter::GetSpatialLayerBitrate(uint64_t nowMs, uint8_t spatialLayer)
	{
		MS_TRACE();

		MS_ASSERT(spatialLayer < this->spatialLayerCounters.size(), "spatialLayer too high");

		uint32_t rate{ 0u };

		// clang-format off
		for (
			uint8_t tIdx{ 0u };
			tIdx < this->spatialLayerCounters[spatialLayer].size();
			++tIdx
		)
		// clang-format on
		{
			auto& temporalLayerCounter = this->spatialLayerCounters[spatialLayer][tIdx];

			rate += temporalLayerCounter.GetBitrate(nowMs);
		}

		return rate;
	}

	uint32_t RtpStreamRecv::TransmissionCounter::GetLayerBitrate(
	  uint64_t nowMs, uint8_t spatialLayer, uint8_t temporalLayer)
	{
		MS_TRACE();

		MS_ASSERT(spatialLayer < this->spatialLayerCounters.size(), "spatialLayer too high");
		MS_ASSERT(
		  temporalLayer < this->spatialLayerCounters[spatialLayer].size(), "temporalLayer too high");

		auto& counter = this->spatialLayerCounters[spatialLayer][temporalLayer];

		return counter.GetBitrate(nowMs);
	}

	size_t RtpStreamRecv::TransmissionCounter::GetPacketCount() const
	{
		MS_TRACE();

		size_t packetCount{ 0u };

		for (auto& spatialLayerCounter : this->spatialLayerCounters)
		{
			for (auto& temporalLayerCounter : spatialLayerCounter)
			{
				packetCount += temporalLayerCounter.GetPacketCount();
			}
		}

		return packetCount;
	}

	size_t RtpStreamRecv::TransmissionCounter::GetBytes() const
	{
		MS_TRACE();

		size_t bytes{ 0u };

		for (const auto& spatialLayerCounter : this->spatialLayerCounters)
		{
			for (const auto& temporalLayerCounter : spatialLayerCounter)
			{
				bytes += temporalLayerCounter.GetBytes();
			}
		}

		return bytes;
	}

	/* Instance methods. */

	RtpStreamRecv::RtpStreamRecv(RTC::RtpStreamRecv::Listener* listener, RTC::RtpStream::Params& params)
	  : RTC::RtpStream::RtpStream(listener, params, 10),
	    transmissionCounter(
	      params.spatialLayers, params.temporalLayers, this->params.useDtx ? 6000 : 2500)
	{
		MS_TRACE();

		if (this->params.useNack)
			this->nackGenerator.reset(new RTC::NackGenerator(this, Settings::configuration.sendNackDelayMs));

		// Run the RTP inactivity periodic timer (use a different timeout if DTX is
		// enabled).
		this->inactivityCheckPeriodicTimer = new Timer(this);
		this->inactive                     = false;

		if (!this->params.useDtx)
			this->inactivityCheckPeriodicTimer->Start(InactivityCheckInterval);
		else
			this->inactivityCheckPeriodicTimer->Start(InactivityCheckIntervalWithDtx);
	}

	RtpStreamRecv::~RtpStreamRecv()
	{
		MS_TRACE();

		// Close the RTP inactivity check periodic timer.
		delete this->inactivityCheckPeriodicTimer;
	}

	void RtpStreamRecv::FillJsonStats(json& jsonObject)
	{
		MS_TRACE();

		uint64_t nowMs = DepLibUV::GetTimeMs();

		RTC::RtpStream::FillJsonStats(jsonObject);

		jsonObject["type"]        = "inbound-rtp";
		jsonObject["jitter"]      = this->jitter;
		jsonObject["packetCount"] = this->transmissionCounter.GetPacketCount();
		jsonObject["byteCount"]   = this->transmissionCounter.GetBytes();
		jsonObject["bitrate"]     = this->transmissionCounter.GetBitrate(nowMs);

		if (GetSpatialLayers() > 1 || GetTemporalLayers() > 1)
		{
			jsonObject["bitrateByLayer"] = json::object();
			auto jsonBitrateByLayerIt    = jsonObject.find("bitrateByLayer");

			for (uint8_t sIdx = 0; sIdx < GetSpatialLayers(); ++sIdx)
			{
				for (uint8_t tIdx = 0; tIdx < GetTemporalLayers(); ++tIdx)
				{
					(*jsonBitrateByLayerIt)[std::to_string(sIdx) + "." + std::to_string(tIdx)] =
					  GetBitrate(nowMs, sIdx, tIdx);
				}
			}
		}
	}

	bool RtpStreamRecv::ReceivePacket(RTC::RtpPacket* packet)
	{
		MS_TRACE();

		// Call the parent method.
		if (!RTC::RtpStream::ReceivePacket(packet))
		{
			MS_WARN_TAG(rtp, "packet discarded");

			return false;
		}

		// Process the packet at codec level.
		if (packet->GetPayloadType() == GetPayloadType())
			RTC::Codecs::Tools::ProcessRtpPacket(packet, GetMimeType());

		// Pass the packet to the NackGenerator.
		if (this->params.useNack)
		{
			// If there is RTX just provide the NackGenerator with the packet.
			if (HasRtx())
			{
				this->nackGenerator->ReceivePacket(packet, /*isRecovered*/ false);
			}
			// If there is no RTX and NackGenerator returns true it means that it
			// was a NACKed packet.
			else if (this->nackGenerator->ReceivePacket(packet, /*isRecovered*/ false))
			{
				// Mark the packet as retransmitted and repaired.
				RTC::RtpStream::PacketRetransmitted(packet);
				RTC::RtpStream::PacketRepaired(packet);
			}
		}

		// Calculate Jitter.
		CalculateJitter(packet->GetTimestamp());

		// Increase transmission counter.
		this->transmissionCounter.Update(packet);

		// Increase media transmission counter.
		this->mediaTransmissionCounter.Update(packet);

		// Not inactive anymore.
		if (this->inactive)
		{
			this->inactive = false;

			ResetScore(10, /*notify*/ true);
		}

		// Restart the inactivityCheckPeriodicTimer.
		if (this->inactivityCheckPeriodicTimer)
			this->inactivityCheckPeriodicTimer->Restart();

		return true;
	}

	bool RtpStreamRecv::ReceiveRtxPacket(RTC::RtpPacket* packet)
	{
		MS_TRACE();

		if (!this->params.useNack)
		{
			MS_WARN_TAG(rtx, "NACK not supported");

			return false;
		}

		MS_ASSERT(packet->GetSsrc() == this->params.rtxSsrc, "invalid ssrc on RTX packet");

		// Check that the payload type corresponds to the one negotiated.
		if (packet->GetPayloadType() != this->params.rtxPayloadType)
		{
			MS_WARN_TAG(
			  rtx,
			  "ignoring RTX packet with invalid payload type [ssrc:%" PRIu32 ", seq:%" PRIu16
			  ", pt:%" PRIu8 "]",
			  packet->GetSsrc(),
			  packet->GetSequenceNumber(),
			  packet->GetPayloadType());

			return false;
		}

		if (HasRtx())
		{
			if (!this->rtxStream->ReceivePacket(packet))
			{
				MS_WARN_TAG(rtx, "RTX packet discarded");

				return false;
			}
		}

#if MS_LOG_DEV_LEVEL == 3
		// Get the RTX packet sequence number for logging purposes.
		auto rtxSeq = packet->GetSequenceNumber();
#endif

		// Get the original RTP packet.
		if (!packet->RtxDecode(this->params.payloadType, this->params.ssrc))
		{
			MS_DEBUG_DEV(
			  "ignoring empty RTX packet [ssrc:%" PRIu32 ", seq:%" PRIu16 ", pt:%" PRIu8 "]",
			  packet->GetSsrc(),
			  packet->GetSequenceNumber(),
			  packet->GetPayloadType());

			return false;
		}

		MS_DEBUG_DEV(
		  "received RTX packet [ssrc:%" PRIu32 ", seq:%" PRIu16 "] recovering original [ssrc:%" PRIu32
		  ", seq:%" PRIu16 "]",
		  this->params.rtxSsrc,
		  rtxSeq,
		  packet->GetSsrc(),
		  packet->GetSequenceNumber());

		// If not a valid packet ignore it.
		if (!RTC::RtpStream::UpdateSeq(packet))
		{
			MS_WARN_TAG(
			  rtx,
			  "invalid RTX packet [ssrc:%" PRIu32 ", seq:%" PRIu16 "]",
			  packet->GetSsrc(),
			  packet->GetSequenceNumber());

			return false;
		}

		// Process the packet at codec level.
		if (packet->GetPayloadType() == GetPayloadType())
			RTC::Codecs::Tools::ProcessRtpPacket(packet, GetMimeType());

		// Mark the packet as retransmitted.
		RTC::RtpStream::PacketRetransmitted(packet);

		// Pass the packet to the NackGenerator and return true just if this was a
		// NACKed packet.
		if (this->nackGenerator->ReceivePacket(packet, /*isRecovered*/ true))
		{
			// Mark the packet as repaired.
			RTC::RtpStream::PacketRepaired(packet);

			// Increase transmission counter.
			this->transmissionCounter.Update(packet);

			// Not inactive anymore.
			if (this->inactive)
			{
				this->inactive = false;

				ResetScore(10, /*notify*/ true);
			}

			// Restart the inactivityCheckPeriodicTimer.
			if (this->inactivityCheckPeriodicTimer)
				this->inactivityCheckPeriodicTimer->Restart();

			return true;
		}

		return false;
	}

	RTC::RTCP::ReceiverReport* RtpStreamRecv::GetRtcpReceiverReport()
	{
		MS_TRACE();

		uint8_t worstRemoteFractionLost{ 0 };

		if (this->params.useInBandFec)
		{
			// Notify the listener so we'll get the worst remote fraction lost.
			static_cast<RTC::RtpStreamRecv::Listener*>(this->listener)
			  ->OnRtpStreamNeedWorstRemoteFractionLost(this, worstRemoteFractionLost);

			if (worstRemoteFractionLost > 0)
				MS_DEBUG_TAG(rtcp, "using worst remote fraction lost:%" PRIu8, worstRemoteFractionLost);
		}

		auto* report = new RTC::RTCP::ReceiverReport();

		report->SetSsrc(GetSsrc());

		uint32_t prevPacketsLost = this->packetsLost;

		// Calculate Packets Expected and Lost.
		auto expected = GetExpectedPackets();

		if (expected > this->mediaTransmissionCounter.GetPacketCount())
			this->packetsLost = expected - this->mediaTransmissionCounter.GetPacketCount();
		else
			this->packetsLost = 0u;

		// Calculate Fraction Lost.
		uint32_t expectedInterval = expected - this->expectedPrior;

		this->expectedPrior = expected;

		uint32_t receivedInterval = this->mediaTransmissionCounter.GetPacketCount() - this->receivedPrior;

		this->receivedPrior = this->mediaTransmissionCounter.GetPacketCount();

		int32_t lostInterval = expectedInterval - receivedInterval;

		if (expectedInterval == 0 || lostInterval <= 0)
			this->fractionLost = 0;
		else
			this->fractionLost = std::round((static_cast<double>(lostInterval << 8) / expectedInterval));

		// Worst remote fraction lost is not worse than local one.
		if (worstRemoteFractionLost <= this->fractionLost)
		{
			this->reportedPacketLost += (this->packetsLost - prevPacketsLost);

			report->SetTotalLost(this->reportedPacketLost);
			report->SetFractionLost(this->fractionLost);
		}
		else
		{
			// Recalculate packetsLost.
			uint32_t newLostInterval = (worstRemoteFractionLost * expectedInterval) >> 8;

			this->reportedPacketLost += newLostInterval;

			report->SetTotalLost(this->reportedPacketLost);
			report->SetFractionLost(worstRemoteFractionLost);
		}

		// Fill the rest of the report.
		report->SetLastSeq(static_cast<uint32_t>(this->maxSeq) + this->cycles);
		report->SetJitter(this->jitter);

		if (this->lastSrReceived != 0)
		{
			// Get delay in milliseconds.
			auto delayMs = static_cast<uint32_t>(DepLibUV::GetTimeMs() - this->lastSrReceived);
			// Express delay in units of 1/65536 seconds.
			uint32_t dlsr = (delayMs / 1000) << 16;

			dlsr |= uint32_t{ (delayMs % 1000) * 65536 / 1000 };

			report->SetDelaySinceLastSenderReport(dlsr);
			report->SetLastSenderReport(this->lastSrTimestamp);
		}
		else
		{
			report->SetDelaySinceLastSenderReport(0);
			report->SetLastSenderReport(0);
		}

		return report;
	}

	RTC::RTCP::ReceiverReport* RtpStreamRecv::GetRtxRtcpReceiverReport()
	{
		MS_TRACE();

		if (HasRtx())
			return this->rtxStream->GetRtcpReceiverReport();

		return nullptr;
	}

	void RtpStreamRecv::ReceiveRtcpSenderReport(RTC::RTCP::SenderReport* report)
	{
		MS_TRACE();

		this->lastSrReceived  = DepLibUV::GetTimeMs();
		this->lastSrTimestamp = report->GetNtpSec() << 16;
		this->lastSrTimestamp += report->GetNtpFrac() >> 16;

		// Update info about last Sender Report.
		Utils::Time::Ntp ntp; // NOLINT(cppcoreguidelines-pro-type-member-init)

		ntp.seconds   = report->GetNtpSec();
		ntp.fractions = report->GetNtpFrac();

		this->lastSenderReportNtpMs = Utils::Time::Ntp2TimeMs(ntp);
		this->lastSenderReporTs     = report->GetRtpTs();

		// Update the score with the current RR.
		UpdateScore();
	}

	void RtpStreamRecv::ReceiveRtxRtcpSenderReport(RTC::RTCP::SenderReport* report)
	{
		MS_TRACE();

		if (HasRtx())
			this->rtxStream->ReceiveRtcpSenderReport(report);
	}

	void RtpStreamRecv::ReceiveRtcpXrDelaySinceLastRr(RTC::RTCP::DelaySinceLastRr::SsrcInfo* ssrcInfo)
	{
		MS_TRACE();

		/* Calculate RTT. */

		// Get the NTP representation of the current timestamp.
		uint64_t nowMs = DepLibUV::GetTimeMs();
		auto ntp       = Utils::Time::TimeMs2Ntp(nowMs);

		// Get the compact NTP representation of the current timestamp.
		uint32_t compactNtp = (ntp.seconds & 0x0000FFFF) << 16;

		compactNtp |= (ntp.fractions & 0xFFFF0000) >> 16;

		uint32_t lastRr = ssrcInfo->GetLastReceiverReport();
		uint32_t dlrr   = ssrcInfo->GetDelaySinceLastReceiverReport();

		// RTT in 1/2^16 second fractions.
		uint32_t rtt{ 0 };

		// If no Receiver Extended Report was received by the remote endpoint yet,
		// ignore lastRr and dlrr values in the Sender Extended Report.
		if (lastRr && dlrr && (compactNtp > dlrr + lastRr))
			rtt = compactNtp - dlrr - lastRr;

		// RTT in milliseconds.
		this->rtt = static_cast<float>(rtt >> 16) * 1000;
		this->rtt += (static_cast<float>(rtt & 0x0000FFFF) / 65536) * 1000;

		if (this->rtt > 0.0f)
			this->hasRtt = true;

		// Tell it to the NackGenerator.
		if (this->params.useNack)
			this->nackGenerator->UpdateRtt(static_cast<uint32_t>(this->rtt));
	}

	void RtpStreamRecv::RequestKeyFrame()
	{
		MS_TRACE();

		if (this->params.usePli)
		{
			MS_DEBUG_2TAGS(rtcp, rtx, "sending PLI [ssrc:%" PRIu32 "]", GetSsrc());

			// Sender SSRC should be 0 since there is no media sender involved, but
			// some implementations like gstreamer will fail to process it otherwise.
			RTC::RTCP::FeedbackPsPliPacket packet(GetSsrc(), GetSsrc());

			packet.Serialize(RTC::RTCP::Buffer);

			this->pliCount++;

			// Notify the listener.
			static_cast<RTC::RtpStreamRecv::Listener*>(this->listener)->OnRtpStreamSendRtcpPacket(this, &packet);
		}
		else if (this->params.useFir)
		{
			MS_DEBUG_2TAGS(rtcp, rtx, "sending FIR [ssrc:%" PRIu32 "]", GetSsrc());

			// Sender SSRC should be 0 since there is no media sender involved, but
			// some implementations like gstreamer will fail to process it otherwise.
			RTC::RTCP::FeedbackPsFirPacket packet(GetSsrc(), GetSsrc());
			auto* item = new RTC::RTCP::FeedbackPsFirItem(GetSsrc(), ++this->firSeqNumber);

			packet.AddItem(item);
			packet.Serialize(RTC::RTCP::Buffer);

			this->firCount++;

			// Notify the listener.
			static_cast<RTC::RtpStreamRecv::Listener*>(this->listener)->OnRtpStreamSendRtcpPacket(this, &packet);
		}
	}

	void RtpStreamRecv::Pause()
	{
		MS_TRACE();

		if (this->inactivityCheckPeriodicTimer)
			this->inactivityCheckPeriodicTimer->Stop();

		if (this->params.useNack)
			this->nackGenerator->Reset();

		// Reset jitter.
		this->transit = 0;
		this->jitter  = 0;
	}

	void RtpStreamRecv::Resume()
	{
		MS_TRACE();

		if (this->inactivityCheckPeriodicTimer && !this->inactive)
			this->inactivityCheckPeriodicTimer->Restart();
	}

	void RtpStreamRecv::CalculateJitter(uint32_t rtpTimestamp)
	{
		MS_TRACE();

		if (this->params.clockRate == 0u)
			return;

		auto transit =
		  static_cast<int>(DepLibUV::GetTimeMs() - (rtpTimestamp * 1000 / this->params.clockRate));
		int d = transit - this->transit;

		// First transit calculation, save and return.
		if (this->transit == 0)
		{
			this->transit = transit;

			return;
		}

		this->transit = transit;

		if (d < 0)
			d = -d;

		this->jitter += (1. / 16.) * (static_cast<double>(d) - this->jitter);
	}

	void RtpStreamRecv::UpdateScore()
	{
		MS_TRACE();

		// Calculate number of packets expected in this interval.
		auto totalExpected = GetExpectedPackets();
		uint32_t expected  = totalExpected - this->expectedPriorScore;

		this->expectedPriorScore = totalExpected;

		// Calculate number of packets received in this interval.
		auto totalReceived = this->mediaTransmissionCounter.GetPacketCount();
		uint32_t received  = totalReceived - this->receivedPriorScore;

		this->receivedPriorScore = totalReceived;

		// Calculate number of packets lost in this interval.
		uint32_t lost;

		if (expected < received)
			lost = 0;
		else
			lost = expected - received;

		// Calculate number of packets repaired in this interval.
		auto totalRepaired = this->packetsRepaired;
		uint32_t repaired  = totalRepaired - this->repairedPriorScore;

		this->repairedPriorScore = totalRepaired;

		// Calculate number of packets retransmitted in this interval.
		auto totatRetransmitted = this->packetsRetransmitted;
		uint32_t retransmitted  = totatRetransmitted - this->retransmittedPriorScore;

		this->retransmittedPriorScore = totatRetransmitted;

		if (this->inactive)
			return;

		// We didn't expect more packets to come.
		if (expected == 0)
		{
			RTC::RtpStream::UpdateScore(10);

			return;
		}

		if (lost > received)
			lost = received;

		if (repaired > lost)
		{
			if (HasRtx())
			{
				repaired = lost;
				retransmitted -= repaired - lost;
			}
			else
			{
				lost = repaired;
			}
		}

#if MS_LOG_DEV_LEVEL == 3
		MS_DEBUG_TAG(
		  score,
		  "[totalExpected:%" PRIu32 ", totalReceived:%zu, totalRepaired:%zu",
		  totalExpected,
		  totalReceived,
		  totalRepaired);

		MS_DEBUG_TAG(
		  score,
		  "fixed values [expected:%" PRIu32 ", received:%" PRIu32 ", lost:%" PRIu32
		  ", repaired:%" PRIu32 ", retransmitted:%" PRIu32,
		  expected,
		  received,
		  lost,
		  repaired,
		  retransmitted);
#endif

		auto repairedRatio  = static_cast<float>(repaired) / static_cast<float>(received);
		auto repairedWeight = std::pow(1 / (repairedRatio + 1), 4);

		MS_ASSERT(retransmitted >= repaired, "repaired packets cannot be more than retransmitted ones");

		if (retransmitted > 0)
			repairedWeight *= static_cast<float>(repaired) / retransmitted;

		lost -= repaired * repairedWeight;

		auto deliveredRatio = static_cast<float>(received - lost) / static_cast<float>(received);
		auto score          = static_cast<uint8_t>(std::round(std::pow(deliveredRatio, 4) * 10));

#if MS_LOG_DEV_LEVEL == 3
		MS_DEBUG_TAG(
		  score,
		  "[deliveredRatio:%f, repairedRatio:%f, repairedWeight:%f, new lost:%" PRIu32 ", score:%" PRIu8
		  "]",
		  deliveredRatio,
		  repairedRatio,
		  repairedWeight,
		  lost,
		  score);
#endif

		RtpStream::UpdateScore(score);
	}

	inline void RtpStreamRecv::OnTimer(Timer* timer)
	{
		MS_TRACE();

		if (timer == this->inactivityCheckPeriodicTimer)
		{
			this->inactive = true;

			if (GetScore() != 0)
			{
				MS_WARN_2TAGS(
				  rtp, score, "RTP inactivity detected, resetting score to 0 [ssrc:%" PRIu32 "]", GetSsrc());
			}

			ResetScore(0, /*notify*/ true);
		}
	}

	inline void RtpStreamRecv::OnNackGeneratorNackRequired(const std::vector<uint16_t>& seqNumbers)
	{
		MS_TRACE();

		MS_ASSERT(this->params.useNack, "NACK required but not supported");

		MS_DEBUG_TAG(
		  rtx,
		  "triggering NACK [ssrc:%" PRIu32 ", first seq:%" PRIu16 ", num packets:%zu]",
		  this->params.ssrc,
		  seqNumbers[0],
		  seqNumbers.size());

		RTC::RTCP::FeedbackRtpNackPacket packet(0, GetSsrc());

		auto it        = seqNumbers.begin();
		const auto end = seqNumbers.end();
		size_t numPacketsRequested{ 0 };

		while (it != end)
		{
			uint16_t seq;
			uint16_t bitmask{ 0 };

			seq = *it;
			++it;

			while (it != end)
			{
				uint16_t shift = *it - seq - 1;

				if (shift > 15)
					break;

				bitmask |= (1 << shift);
				++it;
			}

			auto* nackItem = new RTC::RTCP::FeedbackRtpNackItem(seq, bitmask);

			packet.AddItem(nackItem);

			numPacketsRequested += nackItem->CountRequestedPackets();
		}

		// Ensure that the RTCP packet fits into the RTCP buffer.
		if (packet.GetSize() > RTC::RTCP::BufferSize)
		{
			MS_WARN_TAG(rtx, "cannot send RTCP NACK packet, size too big (%zu bytes)", packet.GetSize());

			return;
		}

		this->nackCount++;
		this->nackPacketCount += numPacketsRequested;

		packet.Serialize(RTC::RTCP::Buffer);

		// Notify the listener.
		static_cast<RTC::RtpStreamRecv::Listener*>(this->listener)->OnRtpStreamSendRtcpPacket(this, &packet);
	}

	inline void RtpStreamRecv::OnNackGeneratorKeyFrameRequired()
	{
		MS_TRACE();

		MS_DEBUG_TAG(rtx, "requesting key frame [ssrc:%" PRIu32 "]", this->params.ssrc);

		RequestKeyFrame();
	}
} // namespace RTC
