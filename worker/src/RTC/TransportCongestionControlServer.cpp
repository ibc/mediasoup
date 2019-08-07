#define MS_CLASS "RTC::TransportCongestionControlServer"
// #define MS_LOG_DEV

#include "RTC/TransportCongestionControlServer.hpp"
#include "Logger.hpp"

namespace RTC
{
	/* Static. */

	static constexpr uint64_t FeedbackSendInterval{ 100u }; // In ms.

	/* Instance methods. */

	TransportCongestionControlServer::TransportCongestionControlServer(
	  RTC::TransportCongestionControlServer::Listener* listener, size_t maxRtcpPacketLen)
	  : listener(listener), maxRtcpPacketLen(maxRtcpPacketLen)
	{
		MS_TRACE();

		// Create a feedback packet.
		this->feedbackPacket.reset(new RTC::RTCP::FeedbackRtpTransportPacket(0, 0));

		// Set initial packet count.
		this->feedbackPacket->SetFeedbackPacketCount(this->feedbackPacketCount);

		// Create the feedback send periodic timer.
		this->feedbackSendPeriodicTimer = new Timer(this);
	}

	TransportCongestionControlServer::~TransportCongestionControlServer()
	{
		MS_TRACE();

		delete this->feedbackSendPeriodicTimer;
	}

	void TransportCongestionControlServer::TransportConnected()
	{
		MS_TRACE();

		this->feedbackSendPeriodicTimer->Start(FeedbackSendInterval, FeedbackSendInterval);
	}

	void TransportCongestionControlServer::TransportDisconnected()
	{
		MS_TRACE();

		this->feedbackSendPeriodicTimer->Stop();
	}

	void TransportCongestionControlServer::IncomingPacket(int64_t arrivalTimeMs, uint16_t wideSeqNumber)
	{
		MS_TRACE();

		// Provide the feedback packet with the RTP packet info. If it fails, send
		// current feedback and add the RTP packet to a new one.
		if (!this->feedbackPacket->AddPacket(wideSeqNumber, arrivalTimeMs, this->maxRtcpPacketLen))
		{
			MS_DEBUG_DEV("RTP packet cannot be added into the feedback packet, sending feedback now");

			SendFeedback();

			// Pass the packet info to the new feedback packet.
			this->feedbackPacket->AddPacket(wideSeqNumber, arrivalTimeMs, this->maxRtcpPacketLen);
		}

		// If the feedback packet is full, send it now.
		if (this->feedbackPacket->IsFull())
		{
			MS_DEBUG_DEV("feedback packet is full, sending feedback now");

			SendFeedback();
		}
	}

	inline void TransportCongestionControlServer::SendFeedback()
	{
		MS_TRACE();

		if (!this->feedbackPacket->IsSerializable())
			return;

		auto lastWideSeqNumber = this->feedbackPacket->GetLastSequenceNumber();
		auto lastTimestamp     = this->feedbackPacket->GetLastTimestamp();

		// Notify the listener.
		this->listener->OnTransportCongestionControlServerSendRtcpPacket(this, this->feedbackPacket.get());

		// Create a new feedback packet.
		this->feedbackPacket.reset(new RTC::RTCP::FeedbackRtpTransportPacket(0, 0));

		// Increment packet count.
		this->feedbackPacket->SetFeedbackPacketCount(++this->feedbackPacketCount);

		// Pass the last packet info (if any) as pre base for the new feedback packet.
		if (lastTimestamp > 0u)
		{
			this->feedbackPacket->AddPacket(lastWideSeqNumber, lastTimestamp, this->maxRtcpPacketLen);
		}
	}

	inline void TransportCongestionControlServer::OnTimer(Timer* timer)
	{
		MS_TRACE();

		if (timer == this->feedbackSendPeriodicTimer)
			SendFeedback();
	}
} // namespace RTC