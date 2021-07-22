#ifndef MS_RTC_NACK_GENERATOR_HPP
#define MS_RTC_NACK_GENERATOR_HPP

#include "common.hpp"
#include "RTC/RtpPacket.hpp"
#include "RTC/SeqManager.hpp"
#include "handles/Timer.hpp"
#include <map>
#include <set>
#include <vector>

namespace RTC
{
	class NackGenerator : public Timer::Listener
	{
	public:
		class Listener
		{
		public:
			virtual ~Listener() = default;

		public:
			virtual void OnNackGeneratorNackRequired(const std::vector<uint16_t>& seqNumbers) = 0;
			virtual void OnNackGeneratorKeyFrameRequired()                                    = 0;
		};

	private:
		struct NackInfo
		{
			NackInfo() = default;
			explicit NackInfo(uint16_t seq, uint16_t sendAtSeq, uint64_t createAtMs)
			  : seq(seq), sendAtSeq(sendAtSeq), createAtMs(createAtMs)
			{
			}

			uint16_t seq{ 0u };
			uint16_t sendAtSeq{ 0u };
			uint64_t sentAtMs{ 0u };
			uint8_t retries{ 0u };
			uint64_t createAtMs{ 0u };
		};

		enum class NackFilter
		{
			SEQ,
			TIME
		};

	public:
		explicit NackGenerator(Listener* listener, uint64_t sendNackDelayMs = 0u);
		~NackGenerator() override;

		bool ReceivePacket(RTC::RtpPacket* packet, bool isRecovered);
		size_t GetNackListLength() const
		{
			return this->nackList.size();
		}
		void UpdateRtt(uint32_t rtt)
		{
			this->rtt = rtt;
		}
		void Reset();

	private:
		void AddPacketsToNackList(uint16_t seqStart, uint16_t seqEnd);
		bool RemoveNackItemsUntilKeyFrame();
		std::vector<uint16_t> GetNackBatch(NackFilter filter);
		void MayRunTimer() const;

		/* Pure virtual methods inherited from Timer::Listener. */
	public:
		void OnTimer(Timer* timer) override;

	private:
		// Passed by argument.
		Listener* listener{ nullptr };
		// Allocated by this.
		Timer* timer{ nullptr };
		// Others.
		std::map<uint16_t, NackInfo, RTC::SeqManager<uint16_t>::SeqLowerThan> nackList;
		std::set<uint16_t, RTC::SeqManager<uint16_t>::SeqLowerThan> keyFrameList;
		std::set<uint16_t, RTC::SeqManager<uint16_t>::SeqLowerThan> recoveredList;
		bool started{ false };
		uint16_t lastSeq{ 0u }; // Seq number of last valid packet.
		uint32_t rtt{ 0u };     // Round trip time (ms).
		uint64_t sendNackDelayMs{ 0u };
	};
} // namespace RTC

#endif
