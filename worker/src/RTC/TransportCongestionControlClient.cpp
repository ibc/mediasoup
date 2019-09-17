#define MS_CLASS "RTC::TransportCongestionControlClient"
#define MS_LOG_DEV // TODO

#include "RTC/TransportCongestionControlClient.hpp"
#include "DepLibUV.hpp"
#include "Logger.hpp"
#include <limits>

namespace RTC
{
	/* Static. */

	static constexpr uint64_t AvailableBitrateEventInterval{ 2000u }; // In ms.

	/* Instance methods. */

	TransportCongestionControlClient::TransportCongestionControlClient(
	  RTC::TransportCongestionControlClient::Listener* listener,
	  RTC::BweType bweType,
	  uint32_t initialAvailableBitrate)
	  : listener(listener), bweType(bweType), initialAvailableBitrate(initialAvailableBitrate)
	{
		MS_TRACE();

		// TODO: Must these factories be static members?

		// TODO: Create predictor factory?

		// TODO: Create controller factory. Let's see.
		webrtc::GoogCcFactoryConfig config;

		config.feedback_only = bweType == RTC::BweType::TRANSPORT_CC;

		this->controllerFactory = new webrtc::GoogCcNetworkControllerFactory(std::move(config));

		webrtc::BitrateConstraints bitrateConfig;

		bitrateConfig.start_bitrate_bps = static_cast<int>(this->initialAvailableBitrate);

		this->rtpTransportControllerSend = new webrtc::RtpTransportControllerSend(
		  this, this->predictorFactory, this->controllerFactory, bitrateConfig);

		this->rtpTransportControllerSend->RegisterTargetTransferRateObserver(this);

		this->probationGenerator = new RTC::RtpProbationGenerator();

		this->pacerTimer = new Timer(this);

		auto delay = static_cast<uint64_t>(
		  this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());

		this->pacerTimer->Start(delay);
	}

	TransportCongestionControlClient::~TransportCongestionControlClient()
	{
		MS_TRACE();

		delete this->predictorFactory;
		this->predictorFactory = nullptr;

		delete this->controllerFactory;
		this->controllerFactory = nullptr;

		delete this->rtpTransportControllerSend;
		this->rtpTransportControllerSend = nullptr;

		delete this->probationGenerator;
		this->probationGenerator = nullptr;

		delete this->pacerTimer;
		this->pacerTimer = nullptr;
	}

	void TransportCongestionControlClient::TransportConnected()
	{
		MS_TRACE();

		this->rtpTransportControllerSend->OnNetworkAvailability(true);
	}

	void TransportCongestionControlClient::TransportDisconnected()
	{
		MS_TRACE();

		this->rtpTransportControllerSend->OnNetworkAvailability(false);
	}

	void TransportCongestionControlClient::InsertPacket(webrtc::RtpPacketSendInfo& packetInfo)
	{
		MS_TRACE();

		this->rtpTransportControllerSend->packet_sender()->InsertPacket(packetInfo.length);
		this->rtpTransportControllerSend->OnAddPacket(packetInfo);

		// // TODO: Testing
		// MS_WARN_DEV("---- delay:%lld" PRIu64,
		// this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());
	}

	webrtc::PacedPacketInfo TransportCongestionControlClient::GetPacingInfo()
	{
		MS_TRACE();

		return this->rtpTransportControllerSend->packet_sender()->GetPacingInfo();
	}

	void TransportCongestionControlClient::PacketSent(webrtc::RtpPacketSendInfo& packetInfo, uint64_t now)
	{
		MS_TRACE();

		// Notify the transport feedback adapter about the sent packet.
		rtc::SentPacket sentPacket(packetInfo.transport_sequence_number, now);
		this->rtpTransportControllerSend->OnSentPacket(sentPacket, packetInfo.length);

		// // TODO: Testing
		// MS_WARN_DEV("---- delay:%lld" PRIu64,
		// this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());
	}

	void TransportCongestionControlClient::ReceiveEstimatedBitrate(uint32_t bitrate)
	{
		MS_TRACE();

		// TODO: REMOVE
		MS_DUMP("--- REMB bitrate:%" PRIu32, bitrate);

		this->rtpTransportControllerSend->OnReceivedEstimatedBitrate(bitrate);
	}

	void TransportCongestionControlClient::ReceiveRtcpReceiverReport(
	  const webrtc::RTCPReportBlock& report, float rtt, uint64_t now)
	{
		MS_TRACE();

		this->rtpTransportControllerSend->OnReceivedRtcpReceiverReport(
		  { report }, static_cast<int64_t>(rtt), static_cast<int64_t>(now));
	}

	void TransportCongestionControlClient::ReceiveRtcpTransportFeedback(
	  const RTC::RTCP::FeedbackRtpTransportPacket* feedback)
	{
		MS_TRACE();

		this->rtpTransportControllerSend->OnTransportFeedback(*feedback);

		// // TODO: Testing
		// MS_WARN_DEV("---- delay:%lld" PRIu64,
		// this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());
	}

	void TransportCongestionControlClient::SetDesiredBitrate(uint32_t desiredBitrate)
	{
		MS_TRACE();

		// TODO (ibc): Too much questions here:
		// * It seems that calling SetClientBitratePreferences() all the time breaks everything
		//   since, if constraints.starting_rate is set, the BWE is reseted to that value
		//   every time!
		// * Also note that the initialAvailableBitrate is already given in the constructor:
		//     bitrateConfig.start_bitrate_bps = static_cast<int>(this->initialAvailableBitrate);
		// * Not even sure if SetClientBitratePreferences() must be called at all.

		auto minBitrate       = 30000u;
		uint32_t startBitrate = std::max<uint32_t>(minBitrate, this->initialAvailableBitrate);
		// Let's increase the max bitrate since it may oscillate.
		uint32_t maxBitrate        = std::max<uint32_t>(startBitrate, desiredBitrate) * 1.25;
		uint32_t maxPaddingBitrate = maxBitrate; // TODO: No idea but we want full maxBitrate.

		// webrtc::TargetRateConstraints constraints;

		// constraints.at_time       = webrtc::Timestamp::ms(DepLibUV::GetTime());
		// constraints.min_data_rate = webrtc::DataRate::bps(minBitrate);
		// constraints.max_data_rate = webrtc::DataRate::bps(maxBitrate);
		// constraints.starting_rate = webrtc::DataRate::bps(startBitrate);

		MS_DEBUG_DEV(
		  "[desiredBitrate:%" PRIu32 ", minBitrate:%" PRIu32 ", startBitrate:%" PRIu32
		  ", maxBitrate:%" PRIu32 "]",
		  desiredBitrate,
		  minBitrate,
		  startBitrate,
		  maxBitrate);

		// TODO: Let's not use this.
		// this->rtpTransportControllerSend->SetClientBitratePreferences(constraints);

		this->rtpTransportControllerSend->SetAllocatedSendBitrateLimits(
		  minBitrate, maxPaddingBitrate, maxBitrate);

		// // TODO: Testing
		// MS_WARN_DEV("---- delay:%lld" PRIu64,
		// this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());
	}

	uint32_t TransportCongestionControlClient::GetAvailableBitrate() const
	{
		MS_TRACE();

		return this->availableBitrate;
	}

	void TransportCongestionControlClient::RescheduleNextAvailableBitrateEvent()
	{
		MS_TRACE();

		this->lastAvailableBitrateEventAt = DepLibUV::GetTime();
	}

	void TransportCongestionControlClient::MayEmitAvailableBitrateEvent(uint32_t previousAvailableBitrate)
	{
		MS_TRACE();

		uint64_t now = DepLibUV::GetTime();
		bool notify{ false };

		// Ignore if first event.
		// NOTE: Otherwise it will make the Transport crash since this event also happens
		// during the constructor of this class.
		if (this->lastAvailableBitrateEventAt == 0u)
		{
			this->lastAvailableBitrateEventAt = now;

			return;
		}

		// Emit if this is the first valid event.
		if (!this->availableBitrateEventCalled)
		{
			this->availableBitrateEventCalled = true;

			notify = true;
		}
		// Emit event if AvailableBitrateEventInterval elapsed.
		else if (now - this->lastAvailableBitrateEventAt >= AvailableBitrateEventInterval)
		{
			notify = true;
		}
		// Also emit the event fast if we detect a high BWE value decrease.
		else if (this->availableBitrate < previousAvailableBitrate * 0.75)
		{
			MS_WARN_TAG(
			  bwe,
			  "high BWE value decrease detected, notifying the listener [now:%" PRIu32 ", before:%" PRIu32
			  "]",
			  this->availableBitrate,
			  previousAvailableBitrate);

			notify = true;
		}

		if (notify)
		{
			MS_DEBUG_DEV("notifying the listener with new available bitrate:%" PRIu32, this->availableBitrate);

			this->lastAvailableBitrateEventAt = now;

			this->listener->OnTransportCongestionControlClientAvailableBitrate(
			  this, this->availableBitrate, previousAvailableBitrate);
		}
	}

	void TransportCongestionControlClient::OnTargetTransferRate(webrtc::TargetTransferRate targetTransferRate)
	{
		MS_TRACE();

		auto previousAvailableBitrate = this->availableBitrate;

		// Update availableBitrate.
		// NOTE: Just in case.
		if (targetTransferRate.target_rate.bps() > std::numeric_limits<uint32_t>::max())
			this->availableBitrate = std::numeric_limits<uint32_t>::max();
		else
			this->availableBitrate = static_cast<uint32_t>(targetTransferRate.target_rate.bps());

		// TODO: This produces lot of logs with the very same availableBitrate, so why is this
		// event called so frequently?
		MS_DEBUG_DEV("new available bitrate:%" PRIu32, this->availableBitrate);

		MayEmitAvailableBitrateEvent(previousAvailableBitrate);
	}

	// Called from PacedSender in order to send probation packets.
	void TransportCongestionControlClient::SendPacket(
	  RTC::RtpPacket* packet, const webrtc::PacedPacketInfo& pacingInfo)
	{
		MS_TRACE();

		// Send the packet.
		this->listener->OnTransportCongestionControlClientSendRtpPacket(this, packet, pacingInfo);

		// // TODO: Testing
		// MS_WARN_DEV("---- delay:%lld" PRIu64,
		// this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());
	}

	RTC::RtpPacket* TransportCongestionControlClient::GeneratePadding(size_t size)
	{
		MS_TRACE();

		return this->probationGenerator->GetNextPacket(size);
	}

	void TransportCongestionControlClient::OnTimer(Timer* timer)
	{
		MS_TRACE();

		if (timer == this->pacerTimer)
		{
			// Time to call PacedSender::Process().
			this->rtpTransportControllerSend->packet_sender()->Process();

			auto delay = static_cast<uint64_t>(
			  this->rtpTransportControllerSend->packet_sender()->TimeUntilNextProcess());

			// TODO: REMOVE
			// MS_DEBUG_DEV("---- delay:%" PRIu64, delay);

			this->pacerTimer->Start(delay);

			MayEmitAvailableBitrateEvent(this->availableBitrate);
		}
	}
} // namespace RTC
