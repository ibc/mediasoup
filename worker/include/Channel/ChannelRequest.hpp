#ifndef MS_CHANNEL_REQUEST_HPP
#define MS_CHANNEL_REQUEST_HPP

#include "common.hpp"
#include <nlohmann/json.hpp>
#include <string>
#include <unordered_map>

using json = nlohmann::json;

namespace Channel
{
	// Avoid cyclic #include problem by declaring classes instead of including
	// the corresponding header files.
	class ChannelSocket;

	class ChannelRequest
	{
	public:
		enum class MethodId
		{
			WORKER_CLOSE = 1,
			WORKER_DUMP,
			WORKER_GET_RESOURCE_USAGE,
			WORKER_UPDATE_SETTINGS,
			WORKER_CREATE_ROUTER,
			ROUTER_CLOSE,
			ROUTER_DUMP,
			ROUTER_CREATE_WEBRTC_TRANSPORT,
			ROUTER_CREATE_PLAIN_TRANSPORT,
			ROUTER_CREATE_PIPE_TRANSPORT,
			ROUTER_CREATE_DIRECT_TRANSPORT,
			ROUTER_CREATE_ACTIVE_SPEAKER_OBSERVER,
			ROUTER_CREATE_AUDIO_LEVEL_OBSERVER,
			TRANSPORT_CLOSE,
			TRANSPORT_DUMP,
			TRANSPORT_GET_STATS,
			TRANSPORT_CONNECT,
			TRANSPORT_SET_MAX_INCOMING_BITRATE,
			TRANSPORT_SET_MAX_OUTGOING_BITRATE,
			TRANSPORT_RESTART_ICE,
			TRANSPORT_PRODUCE,
			TRANSPORT_CONSUME,
			TRANSPORT_PRODUCE_DATA,
			TRANSPORT_CONSUME_DATA,
			TRANSPORT_ENABLE_TRACE_EVENT,
			PRODUCER_CLOSE,
			PRODUCER_DUMP,
			PRODUCER_GET_STATS,
			PRODUCER_PAUSE,
			PRODUCER_RESUME,
			PRODUCER_ENABLE_TRACE_EVENT,
			CONSUMER_CLOSE,
			CONSUMER_DUMP,
			CONSUMER_GET_STATS,
			CONSUMER_PAUSE,
			CONSUMER_RESUME,
			CONSUMER_SET_PREFERRED_LAYERS,
			CONSUMER_SET_PRIORITY,
			CONSUMER_REQUEST_KEY_FRAME,
			CONSUMER_ENABLE_TRACE_EVENT,
			DATA_PRODUCER_CLOSE,
			DATA_PRODUCER_DUMP,
			DATA_PRODUCER_GET_STATS,
			DATA_CONSUMER_CLOSE,
			DATA_CONSUMER_DUMP,
			DATA_CONSUMER_GET_STATS,
			DATA_CONSUMER_GET_BUFFERED_AMOUNT,
			DATA_CONSUMER_SET_BUFFERED_AMOUNT_LOW_THRESHOLD,
			RTP_OBSERVER_CLOSE,
			RTP_OBSERVER_PAUSE,
			RTP_OBSERVER_RESUME,
			RTP_OBSERVER_ADD_PRODUCER,
			RTP_OBSERVER_REMOVE_PRODUCER
		};

	private:
		static std::unordered_map<std::string, MethodId> string2MethodId;

	public:
		ChannelRequest(Channel::ChannelSocket* channel, json& jsonRequest);
		virtual ~ChannelRequest();

		void Accept();
		void Accept(json& data);
		void Error(const char* reason = nullptr);
		void TypeError(const char* reason = nullptr);

	public:
		// Passed by argument.
		Channel::ChannelSocket* channel{ nullptr };
		uint32_t id{ 0u };
		std::string method;
		MethodId methodId;
		json internal;
		json data;
		// Others.
		bool replied{ false };
	};
} // namespace Channel

#endif
