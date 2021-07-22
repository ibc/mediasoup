#define MS_CLASS "Channel::ChannelRequest"
// #define MS_LOG_DEV_LEVEL 3

#include "Channel/ChannelRequest.hpp"
#include "Logger.hpp"
#include "MediaSoupErrors.hpp"
#include "Utils.hpp"

namespace Channel
{
	/* Class variables. */

	// clang-format off
	std::unordered_map<std::string, ChannelRequest::MethodId> ChannelRequest::string2MethodId =
	{
		{ "worker.close",                                ChannelRequest::MethodId::WORKER_CLOSE                                     },
		{ "worker.dump",                                 ChannelRequest::MethodId::WORKER_DUMP                                      },
		{ "worker.getResourceUsage",                     ChannelRequest::MethodId::WORKER_GET_RESOURCE_USAGE                        },
		{ "worker.updateSettings",                       ChannelRequest::MethodId::WORKER_UPDATE_SETTINGS                           },
		{ "worker.createRouter",                         ChannelRequest::MethodId::WORKER_CREATE_ROUTER                             },
		{ "router.close",                                ChannelRequest::MethodId::ROUTER_CLOSE                                     },
		{ "router.dump",                                 ChannelRequest::MethodId::ROUTER_DUMP                                      },
		{ "router.createWebRtcTransport",                ChannelRequest::MethodId::ROUTER_CREATE_WEBRTC_TRANSPORT                   },
		{ "router.createPlainTransport",                 ChannelRequest::MethodId::ROUTER_CREATE_PLAIN_TRANSPORT                    },
		{ "router.createPipeTransport",                  ChannelRequest::MethodId::ROUTER_CREATE_PIPE_TRANSPORT                     },
		{ "router.createDirectTransport",                ChannelRequest::MethodId::ROUTER_CREATE_DIRECT_TRANSPORT                   },
		{ "router.createActiveSpeakerObserver",          ChannelRequest::MethodId::ROUTER_CREATE_ACTIVE_SPEAKER_OBSERVER            },
		{ "router.createAudioLevelObserver",             ChannelRequest::MethodId::ROUTER_CREATE_AUDIO_LEVEL_OBSERVER               },
		{ "transport.close",                             ChannelRequest::MethodId::TRANSPORT_CLOSE                                  },
		{ "transport.dump",                              ChannelRequest::MethodId::TRANSPORT_DUMP                                   },
		{ "transport.getStats",                          ChannelRequest::MethodId::TRANSPORT_GET_STATS                              },
		{ "transport.connect",                           ChannelRequest::MethodId::TRANSPORT_CONNECT                                },
		{ "transport.setMaxIncomingBitrate",             ChannelRequest::MethodId::TRANSPORT_SET_MAX_INCOMING_BITRATE               },
		{ "transport.setMaxOutgoingBitrate",             ChannelRequest::MethodId::TRANSPORT_SET_MAX_OUTGOING_BITRATE               },
		{ "transport.restartIce",                        ChannelRequest::MethodId::TRANSPORT_RESTART_ICE                            },
		{ "transport.produce",                           ChannelRequest::MethodId::TRANSPORT_PRODUCE                                },
		{ "transport.consume",                           ChannelRequest::MethodId::TRANSPORT_CONSUME                                },
		{ "transport.produceData",                       ChannelRequest::MethodId::TRANSPORT_PRODUCE_DATA                           },
		{ "transport.consumeData",                       ChannelRequest::MethodId::TRANSPORT_CONSUME_DATA                           },
		{ "transport.enableTraceEvent",                  ChannelRequest::MethodId::TRANSPORT_ENABLE_TRACE_EVENT                     },
		{ "producer.close",                              ChannelRequest::MethodId::PRODUCER_CLOSE                                   },
		{ "producer.dump",                               ChannelRequest::MethodId::PRODUCER_DUMP                                    },
		{ "producer.getStats",                           ChannelRequest::MethodId::PRODUCER_GET_STATS                               },
		{ "producer.pause",                              ChannelRequest::MethodId::PRODUCER_PAUSE                                   },
		{ "producer.resume" ,                            ChannelRequest::MethodId::PRODUCER_RESUME                                  },
		{ "producer.enableTraceEvent",                   ChannelRequest::MethodId::PRODUCER_ENABLE_TRACE_EVENT                      },
		{ "consumer.close",                              ChannelRequest::MethodId::CONSUMER_CLOSE                                   },
		{ "consumer.dump",                               ChannelRequest::MethodId::CONSUMER_DUMP                                    },
		{ "consumer.getStats",                           ChannelRequest::MethodId::CONSUMER_GET_STATS                               },
		{ "consumer.pause",                              ChannelRequest::MethodId::CONSUMER_PAUSE                                   },
		{ "consumer.resume",                             ChannelRequest::MethodId::CONSUMER_RESUME                                  },
		{ "consumer.setPreferredLayers",                 ChannelRequest::MethodId::CONSUMER_SET_PREFERRED_LAYERS                    },
		{ "consumer.setPriority",                        ChannelRequest::MethodId::CONSUMER_SET_PRIORITY                            },
		{ "consumer.requestKeyFrame",                    ChannelRequest::MethodId::CONSUMER_REQUEST_KEY_FRAME                       },
		{ "consumer.enableTraceEvent",                   ChannelRequest::MethodId::CONSUMER_ENABLE_TRACE_EVENT                      },
		{ "dataProducer.close",                          ChannelRequest::MethodId::DATA_PRODUCER_CLOSE                              },
		{ "dataProducer.dump",                           ChannelRequest::MethodId::DATA_PRODUCER_DUMP                               },
		{ "dataProducer.getStats",                       ChannelRequest::MethodId::DATA_PRODUCER_GET_STATS                          },
		{ "dataConsumer.close",                          ChannelRequest::MethodId::DATA_CONSUMER_CLOSE                              },
		{ "dataConsumer.dump",                           ChannelRequest::MethodId::DATA_CONSUMER_DUMP                               },
		{ "dataConsumer.getStats",                       ChannelRequest::MethodId::DATA_CONSUMER_GET_STATS                          },
		{ "dataConsumer.getBufferedAmount",              ChannelRequest::MethodId::DATA_CONSUMER_GET_BUFFERED_AMOUNT                },
		{ "dataConsumer.setBufferedAmountLowThreshold",  ChannelRequest::MethodId::DATA_CONSUMER_SET_BUFFERED_AMOUNT_LOW_THRESHOLD  },
		{ "rtpObserver.close",                           ChannelRequest::MethodId::RTP_OBSERVER_CLOSE                               },
		{ "rtpObserver.pause",                           ChannelRequest::MethodId::RTP_OBSERVER_PAUSE                               },
		{ "rtpObserver.resume",                          ChannelRequest::MethodId::RTP_OBSERVER_RESUME                              },
		{ "rtpObserver.addProducer",                     ChannelRequest::MethodId::RTP_OBSERVER_ADD_PRODUCER                        },
		{ "rtpObserver.removeProducer",                  ChannelRequest::MethodId::RTP_OBSERVER_REMOVE_PRODUCER                     }
	};
	// clang-format on

	/* Instance methods. */

	ChannelRequest::ChannelRequest(Channel::ChannelSocket* channel, json& jsonRequest)
	  : channel(channel)
	{
		MS_TRACE();

		auto jsonIdIt = jsonRequest.find("id");

		if (jsonIdIt == jsonRequest.end() || !Utils::Json::IsPositiveInteger(*jsonIdIt))
			MS_THROW_ERROR("missing id");

		this->id = jsonIdIt->get<uint32_t>();

		auto jsonMethodIt = jsonRequest.find("method");

		if (jsonMethodIt == jsonRequest.end() || !jsonMethodIt->is_string())
			MS_THROW_ERROR("missing method");

		this->method = jsonMethodIt->get<std::string>();

		auto methodIdIt = ChannelRequest::string2MethodId.find(this->method);

		if (methodIdIt == ChannelRequest::string2MethodId.end())
		{
			Error("unknown method");

			MS_THROW_ERROR("unknown method '%s'", this->method.c_str());
		}

		this->methodId = methodIdIt->second;

		auto jsonInternalIt = jsonRequest.find("internal");

		if (jsonInternalIt != jsonRequest.end() && jsonInternalIt->is_object())
			this->internal = *jsonInternalIt;
		else
			this->internal = json::object();

		auto jsonDataIt = jsonRequest.find("data");

		if (jsonDataIt != jsonRequest.end() && jsonDataIt->is_object())
			this->data = *jsonDataIt;
		else
			this->data = json::object();
	}

	ChannelRequest::~ChannelRequest()
	{
		MS_TRACE();
	}

	void ChannelRequest::Accept()
	{
		MS_TRACE();

		MS_ASSERT(!this->replied, "request already replied");

		this->replied = true;

		json jsonResponse = json::object();

		jsonResponse["id"]       = this->id;
		jsonResponse["accepted"] = true;

		this->channel->Send(jsonResponse);
	}

	void ChannelRequest::Accept(json& data)
	{
		MS_TRACE();

		MS_ASSERT(!this->replied, "request already replied");

		this->replied = true;

		json jsonResponse = json::object();

		jsonResponse["id"]       = this->id;
		jsonResponse["accepted"] = true;

		if (data.is_structured())
			jsonResponse["data"] = data;

		this->channel->Send(jsonResponse);
	}

	void ChannelRequest::Error(const char* reason)
	{
		MS_TRACE();

		MS_ASSERT(!this->replied, "request already replied");

		this->replied = true;

		json jsonResponse = json::object();

		jsonResponse["id"]    = this->id;
		jsonResponse["error"] = "Error";

		if (reason != nullptr)
			jsonResponse["reason"] = reason;

		this->channel->Send(jsonResponse);
	}

	void ChannelRequest::TypeError(const char* reason)
	{
		MS_TRACE();

		MS_ASSERT(!this->replied, "request already replied");

		this->replied = true;

		json jsonResponse = json::object();

		jsonResponse["id"]    = this->id;
		jsonResponse["error"] = "TypeError";

		if (reason != nullptr)
			jsonResponse["reason"] = reason;

		this->channel->Send(jsonResponse);
	}
} // namespace Channel
