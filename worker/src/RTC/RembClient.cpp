#define MS_CLASS "RTC::RembClient"
// #define MS_LOG_DEV

#include "RTC/RembClient.hpp"
#include "DepLibUV.hpp"
#include "Logger.hpp"

namespace RTC
{
	/* Static. */

	static constexpr uint64_t EventInterval{ 2000 };    // In ms.
	static constexpr uint64_t MaxEventInterval{ 5000 }; // In ms.

	/* Instance methods. */

	RembClient::RembClient(RTC::RembClient::Listener* listener, uint32_t initialAvailableBitrate)
	  : listener(listener), initialAvailableBitrate(initialAvailableBitrate),
	    availableBitrate(initialAvailableBitrate), lastEventAt(DepLibUV::GetTime())
	{
		MS_TRACE();
	}

	RembClient::~RembClient()
	{
		MS_TRACE();
	}

	void RembClient::ReceiveRtpPacket(RTC::RtpPacket* packet)
	{
		MS_TRACE();

		this->transmissionCounter.Update(packet);
	}

	void RembClient::ReceiveRembFeedback(RTC::RTCP::FeedbackPsRembPacket* remb)
	{
		MS_TRACE();

		uint64_t now = DepLibUV::GetTime();

		// If we don't have recent data yet, start from here.
		if (!CheckStatus())
		{
			// Update last event time and ensure next event is fired soon.
			this->lastEventAt = now - (0.5 * EventInterval);

			return;
		}
		// Otherwise ensure EventInterval has happened.
		else if ((now - this->lastEventAt) < EventInterval)
		{
			return;
		}

		// Update last event time.
		this->lastEventAt = now;

		auto previousRembBitrate = this->rembBitrate;

		// Update rembBitrate.
		this->rembBitrate = static_cast<uint32_t>(remb->GetBitrate());

		int64_t trend =
		  static_cast<int64_t>(this->rembBitrate) - static_cast<int64_t>(previousRembBitrate);
		uint32_t usedBitrate = this->transmissionCounter.GetBitrate(now);

		// Update available bitrate.
		this->availableBitrate = rembBitrate;

		// If latest rembBitrate is less than initialAvailableBitrate but trend is
		// positive, assume initialAvailableBitrate as available bitrate.
		if (this->rembBitrate < initialAvailableBitrate && trend > 0)
			this->availableBitrate = initialAvailableBitrate;

		if (this->availableBitrate >= usedBitrate)
		{
			uint32_t remainingBitrate = this->availableBitrate - usedBitrate;

			MS_DEBUG_DEV(
			  "usable bitrate [availableBitrate:%" PRIu32 " >= usedBitrate:%" PRIu32
			  ", remainingBitrate:%" PRIu32 "]",
			  this->availableBitrate,
			  usedBitrate,
			  remainingBitrate);

			this->listener->OnRembClientRemainingBitrate(this, remainingBitrate);
		}
		else if (trend > 0)
		{
			MS_DEBUG_DEV(
			  "positive REMB trend [availableBitrate:%" PRIu32 " < usedBitrate:%" PRIu32
			  ", trend:%" PRIi64 "]",
			  this->availableBitrate,
			  usedBitrate,
			  trend);

			// Assume that we can use more bitrate (the trend diff) if rembBitrate is
			// higher than initialAvailableBitrate.
			if (this->rembBitrate > initialAvailableBitrate)
			{
				auto remainingBitrate = static_cast<uint32_t>(trend);

				this->availableBitrate += remainingBitrate;

				this->listener->OnRembClientRemainingBitrate(this, remainingBitrate);
			}
		}
		else
		{
			uint32_t exceedingBitrate = usedBitrate - this->availableBitrate;

			MS_DEBUG_DEV(
			  "exceeding bitrate [availableBitrate:%" PRIu32 " < usedBitrate:%" PRIu32
			  ", exceedingBitrate:%" PRIu32 "]",
			  this->availableBitrate,
			  usedBitrate,
			  exceedingBitrate);

			this->listener->OnRembClientExceedingBitrate(this, exceedingBitrate);
		}
	}

	uint32_t RembClient::GetAvailableBitrate()
	{
		MS_TRACE();

		CheckStatus();

		return this->availableBitrate;
	}

	void RembClient::ResecheduleNextEvent()
	{
		MS_TRACE();

		this->lastEventAt = DepLibUV::GetTime();
	}

	inline bool RembClient::CheckStatus()
	{
		MS_TRACE();

		uint64_t now = DepLibUV::GetTime();

		if ((now - this->lastEventAt) < MaxEventInterval)
		{
			return true;
		}
		else
		{
			this->availableBitrate = this->initialAvailableBitrate;
			this->rembBitrate      = 0;

			return false;
		}
	}
} // namespace RTC
