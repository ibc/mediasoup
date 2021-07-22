#ifndef MS_RTC_ACTIVE_SPEAKER_OBSERVER_HPP
#define MS_RTC_ACTIVE_SPEAKER_OBSERVER_HPP

#include "RTC/RtpObserver.hpp"
#include "handles/Timer.hpp"
#include <nlohmann/json.hpp>
#include <unordered_map>
#include <utility>
#include <vector>

// Implementation of Dominant Speaker Identification for Multipoint
// Videoconferencing by Ilana Volfin and Israel Cohen. This
// implementation uses the RTP Audio Level extension from RFC-6464
// for the input signal.
// This has been ported from DominantSpeakerIdentification.java in Jitsi.
// https://github.com/jitsi/jitsi-utils/blob/master/src/main/java/org/jitsi/utils/dsi/DominantSpeakerIdentification.java
namespace RTC
{
	class ActiveSpeakerObserver : public RTC::RtpObserver, public Timer::Listener
	{
	private:
		class Speaker
		{
		public:
			Speaker();
			void EvalActivityScores();
			double GetActivityScore(int32_t interval);
			void LevelChanged(uint32_t level, uint64_t time);
			void LevelTimedOut();

		private:
			bool ComputeImmediates();
			bool ComputeLongs();
			bool ComputeMediums();
			void EvalImmediateActivityScore();
			void EvalMediumActivityScore();
			void EvalLongActivityScore();
			void UpdateMinLevel(int8_t level);

		public:
			bool paused{ false };
			double immediateActivityScore;
			double mediumActivityScore;
			double longActivityScore;
			uint64_t lastLevelChangeTime{ 0 };

		private:
			uint8_t minLevel;
			uint8_t nextMinLevel;
			uint32_t nextMinLevelWindowLen{ 0 };
			std::vector<uint8_t> immediates;
			std::vector<uint8_t> mediums;
			std::vector<uint8_t> longs;
			std::vector<uint8_t> levels;
		};

		struct ProducerSpeaker
		{
			RTC::Producer* producer;
			Speaker* speaker;
		};

	public:
		ActiveSpeakerObserver(const std::string& id, json& data);
		~ActiveSpeakerObserver() override;

	public:
		void AddProducer(RTC::Producer* producer) override;
		void RemoveProducer(RTC::Producer* producer) override;
		void ReceiveRtpPacket(RTC::Producer* producer, RTC::RtpPacket* packet) override;
		void ProducerPaused(RTC::Producer* producer) override;
		void ProducerResumed(RTC::Producer* producer) override;

	private:
		void Paused() override;
		void Resumed() override;
		void Update();
		bool CalculateActiveSpeaker();
		void TimeoutIdleLevels(uint64_t now);

		/* Pure virtual methods inherited from Timer. */
	protected:
		void OnTimer(Timer* timer) override;

	private:
		static constexpr int relativeSpeachActivitiesLen{ 3 };
		double relativeSpeachActivities[relativeSpeachActivitiesLen];
		std::string dominantId{ "" };
		Timer* periodicTimer{ nullptr };
		uint16_t interval{ 300u };
		std::unordered_map<std::string, struct ProducerSpeaker> mapProducerSpeaker;
		uint64_t lastLevelIdleTime{ 0 };
	};
} // namespace RTC

#endif
