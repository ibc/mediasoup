#define MS_CLASS "RTC::ActiveSpeakerObserver"

#include "RTC/ActiveSpeakerObserver.hpp"
#include "Logger.hpp"
#include "MediaSoupErrors.hpp"
#include "Utils.hpp"
#include "Channel/ChannelNotifier.hpp"
#include "RTC/RtpDictionaries.hpp"

namespace RTC
{
	constexpr uint32_t C1{ 3 };
	constexpr uint32_t C2{ 2 };
	constexpr uint32_t C3{ 0 };
	constexpr uint32_t N1{ 13 };
	constexpr uint32_t N2{ 5 };
	constexpr uint32_t N3{ 10 };
	constexpr uint32_t LongCount{ 1 };
	constexpr uint32_t LevelIdleTimeout{ 40 };
	constexpr uint64_t SpeakerIdleTimeout{ 60 * 60 * 1000 };
	constexpr uint32_t LongThreashold{ 4 };
	constexpr uint32_t MaxLevel{ 127 };
	constexpr uint32_t MinLevel{ 0 };
	constexpr uint32_t MinLevelWindowLen{ 15 * 1000 / 20 };
	constexpr uint32_t MediumThreshold{ 7 };
	constexpr uint32_t SubunitLengthN1{ (MaxLevel - MinLevel + N1 - 1) / N1 };
	constexpr uint32_t ImmediateBuffLen{ LongCount * N3 * N2 };
	constexpr uint32_t MediumsBuffLen{ LongCount * N3 };
	constexpr uint32_t LongsBuffLen{ LongCount };
	constexpr uint32_t LevelsBuffLen{ LongCount * N3 * N2 };
	constexpr double MinActivityScore{ 0.0000000001 };

	inline int64_t BinomialCoefficient(int32_t n, int32_t r)
	{
		int32_t m = n - r;

		if (r < m)
		{
			r = m;
		}

		int64_t t = 1;
		for (int64_t i = n, j = 1; i > r; i--, ++j)
		{
			t = t * i / j;
		}

		return t;
	}

	inline double ComputeActivityScore(
	  const uint8_t vL, const uint32_t nR, const double p, const double lambda)
	{
		double activityScore = std::log(BinomialCoefficient(nR, vL)) + vL * std::log(p) +
		                       (nR - vL) * std::log(1 - p) - std::log(lambda) + lambda * vL;

		if (activityScore < MinActivityScore)
		{
			activityScore = MinActivityScore;
		}

		return activityScore;
	}

	inline bool ComputeBigs(
	  const std::vector<uint8_t>& littles, std::vector<uint8_t>& bigs, uint8_t threashold)
	{
		uint32_t littleLen       = littles.size();
		uint32_t bigLen          = bigs.size();
		uint32_t littleLenPerBig = littleLen / bigLen;
		bool changed             = false;

		for (uint32_t b = 0, l = 0; b < bigLen; b++)
		{
			uint8_t sum = 0;

			for (uint32_t lEnd = l + littleLenPerBig; l < lEnd; ++l)
			{
				if (littles[l] > threashold)
				{
					++sum;
				}
			}

			if (bigs[b] != sum)
			{
				bigs[b] = sum;
				changed = true;
			}
		}

		return changed;
	}

	ActiveSpeakerObserver::ActiveSpeakerObserver(const std::string& id, json& data)
	  : RTC::RtpObserver(id)
	{
		MS_TRACE();

		auto jsonIntervalIt = data.find("interval");

		if (jsonIntervalIt == data.end() || !jsonIntervalIt->is_number())
			MS_THROW_TYPE_ERROR("missing interval");

		this->interval = jsonIntervalIt->get<int16_t>();

		if (this->interval < 100)
			this->interval = 100;
		else if (this->interval > 5000)
			this->interval = 5000;

		this->periodicTimer = new Timer(this);

		this->periodicTimer->Start(interval, interval);
	}

	ActiveSpeakerObserver::~ActiveSpeakerObserver()
	{
		MS_TRACE();

		delete this->periodicTimer;
	}

	void ActiveSpeakerObserver::AddProducer(RTC::Producer* producer)
	{
		MS_TRACE();

		if (producer->GetKind() != RTC::Media::Kind::AUDIO)
			MS_THROW_TYPE_ERROR("not an audio Producer");

		if (this->mapProducerSpeaker.find(producer->id) != this->mapProducerSpeaker.end())
			MS_THROW_ERROR("Producer already in map");

		this->mapProducerSpeaker[producer->id].producer = producer;
		this->mapProducerSpeaker[producer->id].speaker  = new Speaker();
	}

	void ActiveSpeakerObserver::RemoveProducer(RTC::Producer* producer)
	{
		MS_TRACE();

		auto it = this->mapProducerSpeaker.find(producer->id);

		if (it == this->mapProducerSpeaker.end())
		{
			return;
		}

		if (it->second.speaker != nullptr)
		{
			delete it->second.speaker;
		}

		this->mapProducerSpeaker.erase(producer->id);

		if (producer->id == this->dominantId)
		{
			Update();
		}
	}

	void ActiveSpeakerObserver::ProducerResumed(RTC::Producer* producer)
	{
		MS_TRACE();

		auto it = this->mapProducerSpeaker.find(producer->id);

		if (it != this->mapProducerSpeaker.end())
		{
			auto& rtpObserver = it->second;

			rtpObserver.speaker->paused = false;
		}
	}

	void ActiveSpeakerObserver::ProducerPaused(RTC::Producer* producer)
	{
		MS_TRACE();

		auto it = this->mapProducerSpeaker.find(producer->id);

		if (it != this->mapProducerSpeaker.end())
		{
			auto& rtpObserver = it->second;

			rtpObserver.speaker->paused = true;
		}
	}

	void ActiveSpeakerObserver::ReceiveRtpPacket(RTC::Producer* producer, RTC::RtpPacket* packet)
	{
		MS_TRACE();

		if (IsPaused())
			return;

		uint8_t volume;
		bool voice;

		if (!packet->ReadSsrcAudioLevel(volume, voice))
			return;

		auto it = this->mapProducerSpeaker.find(producer->id);

		if (it != this->mapProducerSpeaker.end())
		{
			auto& rtpObserver = it->second;
			uint64_t now      = DepLibUV::GetTimeMs();

			rtpObserver.speaker->LevelChanged(volume, now);
		}
	}

	void ActiveSpeakerObserver::Paused()
	{
		MS_TRACE();

		this->periodicTimer->Stop();
	}

	void ActiveSpeakerObserver::Resumed()
	{
		MS_TRACE();

		this->periodicTimer->Restart();
	}

	void ActiveSpeakerObserver::OnTimer(Timer* timer)
	{
		MS_TRACE();

		Update();
	}

	void ActiveSpeakerObserver::Update()
	{
		MS_TRACE();

		uint64_t now             = DepLibUV::GetTimeMs();
		int64_t levelIdleTimeout = LevelIdleTimeout - (now - this->lastLevelIdleTime);

		if (levelIdleTimeout <= 0)
		{
			if (this->lastLevelIdleTime != 0)
			{
				TimeoutIdleLevels(now);
			}
			this->lastLevelIdleTime = now;
		}

		if (!this->mapProducerSpeaker.empty() && CalculateActiveSpeaker())
		{
			json data          = json::object();
			data["producerId"] = this->dominantId;

			Channel::ChannelNotifier::Emit(this->id, "dominantspeaker", data);
		}
	}

	bool ActiveSpeakerObserver::CalculateActiveSpeaker()
	{
		MS_TRACE();

		std::string newDominantId;
		int32_t speakerCount = this->mapProducerSpeaker.size();

		if (speakerCount == 0)
		{
			newDominantId = "";
		}
		else if (speakerCount == 1)
		{
			auto it = this->mapProducerSpeaker.begin();

			newDominantId = it->second.producer->id;
		}
		else
		{
			Speaker* dominantSpeaker =
			  (this->dominantId.empty()) ? nullptr : this->mapProducerSpeaker[this->dominantId].speaker;

			if (dominantSpeaker == nullptr)
			{
				auto item       = this->mapProducerSpeaker.begin();
				newDominantId   = item->first;
				dominantSpeaker = item->second.speaker;
			}
			else
			{
				newDominantId = "";
			}

			dominantSpeaker->EvalActivityScores();
			double newDominantC2 = C2;

			for (auto it = this->mapProducerSpeaker.begin(); it != this->mapProducerSpeaker.end(); ++it)
			{
				Speaker* speaker      = it->second.speaker;
				const std::string& id = it->second.producer->id;

				if (id == this->dominantId || speaker->paused)
				{
					continue;
				}

				speaker->EvalActivityScores();

				for (int interval = 0; interval < this->relativeSpeachActivitiesLen; ++interval)
				{
					this->relativeSpeachActivities[interval] = std::log(
					  dominantSpeaker->GetActivityScore(interval) / speaker->GetActivityScore(interval));
				}

				double c1 = this->relativeSpeachActivities[0];
				double c2 = this->relativeSpeachActivities[1];
				double c3 = this->relativeSpeachActivities[2];

				if ((c1 > C1) && (c2 > C2) && (c3 > C3) && (c2 > newDominantC2))
				{
					newDominantC2 = c2;
					newDominantId = id;
				}
			}
		}

		if (!newDominantId.empty() && newDominantId != this->dominantId)
		{
			this->dominantId = newDominantId;

			return true;
		}

		return false;
	}

	void ActiveSpeakerObserver::TimeoutIdleLevels(uint64_t now)
	{
		MS_TRACE();

		for (auto it = this->mapProducerSpeaker.begin(); it != this->mapProducerSpeaker.end(); ++it)
		{
			Speaker* speaker      = it->second.speaker;
			const std::string& id = it->second.producer->id;
			uint64_t idle         = now - speaker->lastLevelChangeTime;

			if (SpeakerIdleTimeout < idle && (this->dominantId.empty() || id != this->dominantId))
			{
				speaker->paused = true;
			}
			else if (LevelIdleTimeout < idle)
			{
				speaker->LevelTimedOut();
			}
		}
	}

	ActiveSpeakerObserver::Speaker::Speaker()
	{
		MS_TRACE();

		this->minLevel               = MinLevel;
		this->nextMinLevel           = MinLevel;
		this->immediateActivityScore = MinActivityScore;
		this->mediumActivityScore    = MinActivityScore;
		this->longActivityScore      = MinActivityScore;

		this->immediates.resize(ImmediateBuffLen);
		this->mediums.resize(MediumsBuffLen);
		this->longs.resize(LongsBuffLen);
		this->levels.resize(LevelsBuffLen);

		this->lastLevelChangeTime = DepLibUV::GetTimeMs();
	}

	void ActiveSpeakerObserver::Speaker::EvalActivityScores()
	{
		MS_TRACE();

		if (ComputeImmediates())
		{
			EvalImmediateActivityScore();

			if (ComputeMediums())
			{
				EvalMediumActivityScore();

				if (ComputeLongs())
				{
					EvalLongActivityScore();
				}
			}
		}
	}

	double ActiveSpeakerObserver::Speaker::GetActivityScore(int32_t interval)
	{
		MS_TRACE();

		switch (interval)
		{
			case 0:
				return this->immediateActivityScore;
			case 1:
				return this->mediumActivityScore;
			case 2:
				return this->longActivityScore;
			default:
				MS_ABORT("interval is invalid");
		}

		return 0;
	}

	void ActiveSpeakerObserver::Speaker::LevelChanged(uint32_t level, uint64_t time)
	{
		if (this->lastLevelChangeTime <= time)
		{
			this->lastLevelChangeTime = time;

			int8_t b = 0;

			if (level < MinLevel)
			{
				b = MinLevel;
			}
			else if (level > MaxLevel)
			{
				b = MaxLevel;
			}
			else
			{
				b = level;
			}

			std::copy(this->levels.begin(), this->levels.end() - 1, this->levels.begin() + 1);

			this->levels[0] = b;
			UpdateMinLevel(b);
			this->paused = false;
		}
	}

	void ActiveSpeakerObserver::Speaker::LevelTimedOut()
	{
		MS_TRACE();

		LevelChanged(MinLevel, this->lastLevelChangeTime);
	}

	bool ActiveSpeakerObserver::Speaker::ComputeImmediates()
	{
		MS_TRACE();

		int8_t minLevel = this->minLevel + SubunitLengthN1;
		bool changed    = false;

		for (uint32_t i = 0; i < ImmediateBuffLen; ++i)
		{
			uint8_t level = this->levels[i];

			if (level < minLevel)
			{
				level = MinLevel;
			}

			uint8_t immediate = (level / SubunitLengthN1);

			if (this->immediates[i] != immediate)
			{
				this->immediates[i] = immediate;
				changed             = true;
			}
		}

		return changed;
	}

	bool ActiveSpeakerObserver::Speaker::ComputeMediums()
	{
		MS_TRACE();

		return ComputeBigs(this->immediates, this->mediums, MediumThreshold);
	}

	bool ActiveSpeakerObserver::Speaker::ComputeLongs()
	{
		MS_TRACE();

		return ComputeBigs(this->mediums, this->longs, LongThreashold);
	}

	void ActiveSpeakerObserver::Speaker::EvalImmediateActivityScore()
	{
		MS_TRACE();

		this->immediateActivityScore = ComputeActivityScore(this->immediates[0], N1, 0.5, 0.78);
	}

	void ActiveSpeakerObserver::Speaker::EvalMediumActivityScore()
	{
		MS_TRACE();

		this->mediumActivityScore = ComputeActivityScore(this->mediums[0], N2, 0.5, 24);
	}

	void ActiveSpeakerObserver::Speaker::EvalLongActivityScore()
	{
		MS_TRACE();

		this->longActivityScore = ComputeActivityScore(this->longs[0], N3, 0.5, 47);
	}

	void ActiveSpeakerObserver::Speaker::UpdateMinLevel(int8_t level)
	{
		MS_TRACE();

		if (level == MinLevel)
		{
			return;
		}

		if ((this->minLevel == MinLevel) || (this->minLevel > level))
		{
			this->minLevel              = level;
			this->nextMinLevel          = MinLevel;
			this->nextMinLevelWindowLen = 0;
		}
		else
		{
			if (this->nextMinLevel == MinLevel)
			{
				this->nextMinLevel          = level;
				this->nextMinLevelWindowLen = 1;
			}
			else
			{
				if (this->nextMinLevel > level)
				{
					this->nextMinLevel = level;
				}
				this->nextMinLevelWindowLen++;

				if (this->nextMinLevelWindowLen >= MinLevelWindowLen)
				{
					double newMinLevel = std::sqrt(static_cast<double>(this->minLevel * this->nextMinLevel));

					if (newMinLevel < MinLevel)
					{
						newMinLevel = MinLevel;
					}
					else if (newMinLevel > MaxLevel)
					{
						newMinLevel = MaxLevel;
					}

					this->minLevel = static_cast<int8_t>(newMinLevel);

					this->nextMinLevel          = MinLevel;
					this->nextMinLevelWindowLen = 0;
				}
			}
		}
	}
} // namespace RTC
