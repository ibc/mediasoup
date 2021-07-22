#define MS_CLASS "RTC::NackGenerator"
// #define MS_LOG_DEV_LEVEL 3

#include "RTC/NackGenerator.hpp"
#include "DepLibUV.hpp"
#include "Logger.hpp"
#include <iterator> // std::ostream_iterator
#include <sstream>  // std::ostringstream
#include <utility>  // std::make_pair()

namespace RTC
{
	/* Static. */

	constexpr size_t MaxPacketAge{ 10000u };
	constexpr size_t MaxNackPackets{ 1000u };
	constexpr uint32_t DefaultRtt{ 100u };
	constexpr uint8_t MaxNackRetries{ 10u };
	constexpr uint64_t TimerInterval{ 40u };

	/* Instance methods. */

	NackGenerator::NackGenerator(Listener* listener) : listener(listener), rtt(DefaultRtt)
	{
		MS_TRACE();

		// Set the timer.
		this->timer = new Timer(this);
	}

	NackGenerator::~NackGenerator()
	{
		MS_TRACE();

		// Close the timer.
		delete this->timer;
	}

	// Returns true if this is a found nacked packet. False otherwise.
	bool NackGenerator::ReceivePacket(RTC::RtpPacket* packet, bool isRecovered)
	{
		MS_TRACE();

		uint16_t seq    = packet->GetSequenceNumber();
		bool isKeyFrame = packet->IsKeyFrame();

		if (!this->started)
		{
			this->started = true;
			this->lastSeq = seq;

			if (isKeyFrame)
				this->keyFrameList.insert(seq);

			return false;
		}

		// Obviously never nacked, so ignore.
		if (seq == this->lastSeq)
			return false;

		// May be an out of order packet, or already handled retransmitted packet,
		// or a retransmitted packet.
		if (SeqManager<uint16_t>::IsSeqLowerThan(seq, this->lastSeq))
		{
			auto it = this->nackList.find(seq);

			// It was a nacked packet.
			if (it != this->nackList.end())
			{
				MS_DEBUG_DEV(
				  "NACKed packet received [ssrc:%" PRIu32 ", seq:%" PRIu16 ", recovered:%s]",
				  packet->GetSsrc(),
				  packet->GetSequenceNumber(),
				  isRecovered ? "true" : "false");

				this->nackList.erase(it);

				return true;
			}

			// Out of order packet or already handled NACKed packet.
			if (!isRecovered)
			{
				MS_WARN_DEV(
				  "ignoring older packet not present in the NACK list [ssrc:%" PRIu32 ", seq:%" PRIu16 "]",
				  packet->GetSsrc(),
				  packet->GetSequenceNumber());
				return false;
			}

			return true;
		}

		// If we are here it means that we may have lost some packets so seq is
		// newer than the latest seq seen.

		if (isKeyFrame)
			this->keyFrameList.insert(seq);

		// Remove old keyframes.
		{
			auto it = this->keyFrameList.lower_bound(seq - MaxPacketAge);

			if (it != this->keyFrameList.begin())
				this->keyFrameList.erase(this->keyFrameList.begin(), it);
		}

		if (isRecovered)
		{
			this->recoveredList.insert(seq);

			// Remove old ones so we don't accumulate recovered packets.
			auto it = this->recoveredList.lower_bound(seq - MaxPacketAge);

			if (it != this->recoveredList.begin())
				this->recoveredList.erase(this->recoveredList.begin(), it);

			// Do not let a packet pass if it's newer than last seen seq and came via
			// RTX.
			return true;
		}

		AddPacketsToNackList(this->lastSeq + 1, seq);

		this->lastSeq = seq;

		// Check if there are any nacks that are waiting for this seq number.
		std::vector<uint16_t> nackBatch = GetNackBatch(NackFilter::SEQ);

		if (!nackBatch.empty())
			this->listener->OnNackGeneratorNackRequired(nackBatch);

		// This is important. Otherwise the running timer (filter:TIME) would be
		// interrupted and NACKs would never been sent more than once for each seq.
		if (!this->timer->IsActive())
			MayRunTimer();

		return false;
	}

	void NackGenerator::AddPacketsToNackList(uint16_t seqStart, uint16_t seqEnd)
	{
		MS_TRACE();

		// Remove old packets.
		auto it = this->nackList.lower_bound(seqEnd - MaxPacketAge);

		this->nackList.erase(this->nackList.begin(), it);

		// If the nack list is too large, remove packets from the nack list until
		// the latest first packet of a keyframe. If the list is still too large,
		// clear it and request a keyframe.
		uint16_t numNewNacks = seqEnd - seqStart;

		if (this->nackList.size() + numNewNacks > MaxNackPackets)
		{
			// clang-format off
			while (
				RemoveNackItemsUntilKeyFrame() &&
				this->nackList.size() + numNewNacks > MaxNackPackets
			)
			// clang-format on
			{
			}

			if (this->nackList.size() + numNewNacks > MaxNackPackets)
			{
				MS_WARN_TAG(
				  rtx, "NACK list full, clearing it and requesting a key frame [seqEnd:%" PRIu16 "]", seqEnd);

				this->nackList.clear();
				this->listener->OnNackGeneratorKeyFrameRequired();

				return;
			}
		}

		for (uint16_t seq = seqStart; seq != seqEnd; ++seq)
		{
			MS_ASSERT(this->nackList.find(seq) == this->nackList.end(), "packet already in the NACK list");

			// Do not send NACK for packets that are already recovered by RTX.
			if (this->recoveredList.find(seq) != this->recoveredList.end())
				continue;

			this->nackList.emplace(std::make_pair(seq, NackInfo{ seq, seq }));
		}
	}

	bool NackGenerator::RemoveNackItemsUntilKeyFrame()
	{
		MS_TRACE();

		while (!this->keyFrameList.empty())
		{
			auto it = this->nackList.lower_bound(*this->keyFrameList.begin());

			if (it != this->nackList.begin())
			{
				// We have found a keyframe that actually is newer than at least one
				// packet in the nack list.
				this->nackList.erase(this->nackList.begin(), it);

				return true;
			}

			// If this keyframe is so old it does not remove any packets from the list,
			// remove it from the list of keyframes and try the next keyframe.
			this->keyFrameList.erase(this->keyFrameList.begin());
		}

		return false;
	}

	std::vector<uint16_t> NackGenerator::GetNackBatch(NackFilter filter)
	{
		MS_TRACE();

		uint64_t nowMs = DepLibUV::GetTimeMs();
		std::vector<uint16_t> nackBatch;

		auto it = this->nackList.begin();

		while (it != this->nackList.end())
		{
			NackInfo& nackInfo = it->second;
			uint16_t seq       = nackInfo.seq;

			// clang-format off
			if (
				filter == NackFilter::SEQ &&
				nackInfo.sentAtMs == 0 &&
				(
					nackInfo.sendAtSeq == this->lastSeq ||
					SeqManager<uint16_t>::IsSeqHigherThan(this->lastSeq, nackInfo.sendAtSeq)
				)
			)
			// clang-format on
			{
				nackBatch.emplace_back(seq);
				nackInfo.retries++;
				nackInfo.sentAtMs = nowMs;

				if (nackInfo.retries >= MaxNackRetries)
				{
					MS_WARN_TAG(
					  rtx,
					  "sequence number removed from the NACK list due to max retries [filter:seq, seq:%" PRIu16
					  "]",
					  seq);

					it = this->nackList.erase(it);
				}
				else
				{
					++it;
				}

				continue;
			}

			if (filter == NackFilter::TIME && nowMs - nackInfo.sentAtMs >= this->rtt)
			{
				nackBatch.emplace_back(seq);
				nackInfo.retries++;
				nackInfo.sentAtMs = nowMs;

				if (nackInfo.retries >= MaxNackRetries)
				{
					MS_WARN_TAG(
					  rtx,
					  "sequence number removed from the NACK list due to max retries [filter:time, seq:%" PRIu16
					  "]",
					  seq);

					it = this->nackList.erase(it);
				}
				else
				{
					++it;
				}

				continue;
			}

			++it;
		}

#if MS_LOG_DEV_LEVEL == 3
		if (!nackBatch.empty())
		{
			std::ostringstream seqsStream;
			std::copy(
			  nackBatch.begin(), nackBatch.end() - 1, std::ostream_iterator<uint32_t>(seqsStream, ","));
			seqsStream << nackBatch.back();

			if (filter == NackFilter::SEQ)
				MS_DEBUG_DEV("[filter:SEQ, asking seqs:%s]", seqsStream.str().c_str());
			else
				MS_DEBUG_DEV("[filter:TIME, asking seqs:%s]", seqsStream.str().c_str());
		}
#endif

		return nackBatch;
	}

	void NackGenerator::Reset()
	{
		MS_TRACE();

		this->nackList.clear();
		this->keyFrameList.clear();
		this->recoveredList.clear();

		this->started = false;
		this->lastSeq = 0u;
	}

	inline void NackGenerator::MayRunTimer() const
	{
		if (!this->nackList.empty())
			this->timer->Start(TimerInterval);
	}

	inline void NackGenerator::OnTimer(Timer* /*timer*/)
	{
		MS_TRACE();

		std::vector<uint16_t> nackBatch = GetNackBatch(NackFilter::TIME);

		if (!nackBatch.empty())
			this->listener->OnNackGeneratorNackRequired(nackBatch);

		MayRunTimer();
	}
} // namespace RTC
