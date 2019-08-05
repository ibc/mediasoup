#ifndef MS_RTC_TRANSPORT_HPP
#define MS_RTC_TRANSPORT_HPP

#include "common.hpp"
#include "DepLibUV.hpp"
#include "json.hpp"
#include "Channel/Request.hpp"
#include "RTC/Consumer.hpp"
#include "RTC/DataConsumer.hpp"
#include "RTC/DataProducer.hpp"
#include "RTC/Producer.hpp"
#include "RTC/RTCP/CompoundPacket.hpp"
#include "RTC/RTCP/Packet.hpp"
#include "RTC/RTCP/ReceiverReport.hpp"
#include "RTC/RateCalculator.hpp"
#include "RTC/RembClient.hpp"
#include "RTC/RembServer/RemoteBitrateEstimatorAbsSendTime.hpp"
#include "RTC/RtpHeaderExtensionIds.hpp"
#include "RTC/RtpListener.hpp"
#include "RTC/RtpPacket.hpp"
#include "RTC/SctpAssociation.hpp"
#include "RTC/SctpListener.hpp"
#include "RTC/TransportCongestionControlServer.hpp"
#include "handles/Timer.hpp"
#include <string>
#include <unordered_map>

using json = nlohmann::json;

namespace RTC
{
	class Transport : public RTC::Producer::Listener,
	                  public RTC::Consumer::Listener,
	                  public RTC::DataProducer::Listener,
	                  public RTC::DataConsumer::Listener,
	                  public RTC::SctpAssociation::Listener,
	                  public RTC::RembClient::Listener,
	                  public RTC::RembServer::RemoteBitrateEstimator::Listener,
	                  public RTC::TransportCongestionControlServer::Listener,
	                  public Timer::Listener
	{
	public:
		class Listener
		{
		public:
			virtual void OnTransportNewProducer(RTC::Transport* transport, RTC::Producer* producer) = 0;
			virtual void OnTransportProducerClosed(RTC::Transport* transport, RTC::Producer* producer) = 0;
			virtual void OnTransportProducerPaused(RTC::Transport* transport, RTC::Producer* producer) = 0;
			virtual void OnTransportProducerResumed(RTC::Transport* transport, RTC::Producer* producer) = 0;
			virtual void OnTransportProducerNewRtpStream(
			  RTC::Transport* transport,
			  RTC::Producer* producer,
			  RTC::RtpStream* rtpStream,
			  uint32_t mappedSsrc) = 0;
			virtual void OnTransportProducerRtpStreamScore(
			  RTC::Transport* transport,
			  RTC::Producer* producer,
			  RTC::RtpStream* rtpStream,
			  uint8_t score,
			  uint8_t previousScore) = 0;
			virtual void OnTransportProducerRtcpSenderReport(
			  RTC::Transport* transport, RTC::Producer* producer, RTC::RtpStream* rtpStream, bool first) = 0;
			virtual void OnTransportProducerRtpPacketReceived(
			  RTC::Transport* transport, RTC::Producer* producer, RTC::RtpPacket* packet) = 0;
			virtual void OnTransportNeedWorstRemoteFractionLost(
			  RTC::Transport* transport,
			  RTC::Producer* producer,
			  uint32_t mappedSsrc,
			  uint8_t& worstRemoteFractionLost) = 0;
			virtual void OnTransportNewConsumer(
			  RTC::Transport* transport, RTC::Consumer* consumer, std::string& producerId) = 0;
			virtual void OnTransportConsumerClosed(RTC::Transport* transport, RTC::Consumer* consumer) = 0;
			virtual void OnTransportConsumerProducerClosed(
			  RTC::Transport* transport, RTC::Consumer* consumer) = 0;
			virtual void OnTransportConsumerKeyFrameRequested(
			  RTC::Transport* transport, RTC::Consumer* consumer, uint32_t mappedSsrc) = 0;
			virtual void OnTransportNewDataProducer(
			  RTC::Transport* transport, RTC::DataProducer* dataProducer) = 0;
			virtual void OnTransportDataProducerClosed(
			  RTC::Transport* transport, RTC::DataProducer* dataProducer) = 0;
			virtual void OnTransportDataProducerSctpMessageReceived(
			  RTC::Transport* transport,
			  RTC::DataProducer* dataProducer,
			  uint32_t ppid,
			  const uint8_t* msg,
			  size_t len) = 0;
			virtual void OnTransportNewDataConsumer(
			  RTC::Transport* transport, RTC::DataConsumer* dataConsumer, std::string& dataProducerId) = 0;
			virtual void OnTransportDataConsumerClosed(
			  RTC::Transport* transport, RTC::DataConsumer* dataConsumer) = 0;
			virtual void OnTransportDataConsumerDataProducerClosed(
			  RTC::Transport* transport, RTC::DataConsumer* dataConsumer) = 0;
		};

	public:
		Transport(const std::string& id, Listener* listener, json& data);
		virtual ~Transport();

	public:
		void CloseProducersAndConsumers();
		// Subclasses must also invoke the parent Close().
		virtual void FillJson(json& jsonObject) const;
		virtual void FillJsonStats(json& jsonArray);
		// Subclasses must implement this method and call the parent's one to
		// handle common requests.
		virtual void HandleRequest(Channel::Request* request);

	protected:
		// Must be called from the subclass.
		void Connected();
		void Disconnected();
		void DataReceived(size_t len);
		void DataSent(size_t len);
		void ReceiveRtpPacket(RTC::RtpPacket* packet);
		void ReceiveRtcpPacket(RTC::RTCP::Packet* packet);
		void ReceiveSctpData(const uint8_t* data, size_t len);

	private:
		void SetNewProducerIdFromRequest(Channel::Request* request, std::string& producerId) const;
		RTC::Producer* GetProducerFromRequest(Channel::Request* request) const;
		void SetNewConsumerIdFromRequest(Channel::Request* request, std::string& consumerId) const;
		RTC::Consumer* GetConsumerFromRequest(Channel::Request* request) const;
		RTC::Consumer* GetConsumerByMediaSsrc(uint32_t ssrc) const;
		void SetNewDataProducerIdFromRequest(Channel::Request* request, std::string& dataProducerId) const;
		RTC::DataProducer* GetDataProducerFromRequest(Channel::Request* request) const;
		void SetNewDataConsumerIdFromRequest(Channel::Request* request, std::string& dataConsumerId) const;
		RTC::DataConsumer* GetDataConsumerFromRequest(Channel::Request* request) const;
		virtual bool IsConnected() const                   = 0;
		virtual void SendRtpPacket(RTC::RtpPacket* packet) = 0;
		void HandleRtcpPacket(RTC::RTCP::Packet* packet);
		void SendRtcp(uint64_t now);
		virtual void SendRtcpPacket(RTC::RTCP::Packet* packet)                 = 0;
		virtual void SendRtcpCompoundPacket(RTC::RTCP::CompoundPacket* packet) = 0;
		virtual void SendSctpData(const uint8_t* data, size_t len)             = 0;
		void DistributeAvailableOutgoingBitrate();
		void MayRunRembLimiterTimer();

		/* Pure virtual methods inherited from RTC::Producer::Listener. */
	public:
		void OnProducerPaused(RTC::Producer* producer) override;
		void OnProducerResumed(RTC::Producer* producer) override;
		void OnProducerNewRtpStream(
		  RTC::Producer* producer, RTC::RtpStream* rtpStream, uint32_t mappedSsrc) override;
		void OnProducerRtpStreamScore(
		  RTC::Producer* producer, RTC::RtpStream* rtpStream, uint8_t score, uint8_t previousScore) override;
		void OnProducerRtcpSenderReport(
		  RTC::Producer* producer, RTC::RtpStream* rtpStream, bool first) override;
		void OnProducerRtpPacketReceived(RTC::Producer* producer, RTC::RtpPacket* packet) override;
		void OnProducerSendRtcpPacket(RTC::Producer* producer, RTC::RTCP::Packet* packet) override;
		void OnProducerNeedWorstRemoteFractionLost(
		  RTC::Producer* producer, uint32_t mappedSsrc, uint8_t& worstRemoteFractionLost) override;

		/* Pure virtual methods inherited from RTC::Consumer::Listener. */
	public:
		void OnConsumerSendRtpPacket(RTC::Consumer* consumer, RTC::RtpPacket* packet) override;
		void OnConsumerRetransmitRtpPacket(RTC::Consumer* consumer, RTC::RtpPacket* packet) override;
		void OnConsumerKeyFrameRequested(RTC::Consumer* consumer, uint32_t mappedSsrc) override;
		void OnConsumerNeedBitrateChange(RTC::Consumer* consumer) override;
		void OnConsumerProducerClosed(RTC::Consumer* consumer) override;

		/* Pure virtual methods inherited from RTC::DataProducer::Listener. */
	public:
		void OnDataProducerSctpMessageReceived(
		  RTC::DataProducer* dataProducer, uint32_t ppid, const uint8_t* msg, size_t len) override;

		/* Pure virtual methods inherited from RTC::DataConsumer::Listener. */
	public:
		void OnDataConsumerSendSctpMessage(
		  RTC::DataConsumer* dataConsumer, uint32_t ppid, const uint8_t* msg, size_t len) override;
		void OnDataConsumerDataProducerClosed(RTC::DataConsumer* dataConsumer) override;

		/* Pure virtual methods inherited from RTC::SctpAssociation::Listener. */
	public:
		void OnSctpAssociationConnecting(RTC::SctpAssociation* sctpAssociation) override;
		void OnSctpAssociationConnected(RTC::SctpAssociation* sctpAssociation) override;
		void OnSctpAssociationFailed(RTC::SctpAssociation* sctpAssociation) override;
		void OnSctpAssociationClosed(RTC::SctpAssociation* sctpAssociation) override;
		void OnSctpAssociationSendData(
		  RTC::SctpAssociation* sctpAssociation, const uint8_t* data, size_t len) override;
		void OnSctpAssociationMessageReceived(
		  RTC::SctpAssociation* sctpAssociation,
		  uint16_t streamId,
		  uint32_t ppid,
		  const uint8_t* msg,
		  size_t len) override;

		/* Pure virtual methods inherited from RTC::RembClient::Listener. */
	public:
		void OnRembClientAvailableBitrate(RTC::RembClient* rembClient, uint32_t availableBitrate) override;
		void OnRembClientNeedProbationBitrate(RTC::RembClient* rembClient, uint32_t& probationBitrate) override;
		void OnRembClientSendProbationRtpPacket(RTC::RembClient* rembClient, RTC::RtpPacket* packet) override;

		/* Pure virtual methods inherited from RTC::RembServer::RemoteBitrateEstimator::Listener. */
	public:
		void OnRembServerAvailableBitrate(
		  const RTC::RembServer::RemoteBitrateEstimator* remoteBitrateEstimator,
		  const std::vector<uint32_t>& ssrcs,
		  uint32_t availableBitrate) override;

		/* Pure virtual methods inherited from RTC::TransportCongestionControlServer::Listener. */
	public:
		void OnTransportCongestionControlServerSendRtcpPacket(
		  RTC::TransportCongestionControlServer* tccServer, RTC::RTCP::Packet* packet) override;

		/* Pure virtual methods inherited from Timer::Listener. */
	public:
		void OnTimer(Timer* timer) override;

	public:
		// Passed by argument.
		const std::string id;

	private:
		// Passed by argument.
		Listener* listener{ nullptr };
		// Allocated by this.
		std::unordered_map<std::string, RTC::Producer*> mapProducers;
		std::unordered_map<std::string, RTC::Consumer*> mapConsumers;
		std::unordered_map<std::string, RTC::DataProducer*> mapDataProducers;
		std::unordered_map<std::string, RTC::DataConsumer*> mapDataConsumers;
		std::unordered_map<uint32_t, RTC::Consumer*> mapSsrcConsumer;
		RTC::SctpAssociation* sctpAssociation{ nullptr };
		RTC::RembClient* rembClient{ nullptr };
		RTC::RembServer::RemoteBitrateEstimatorAbsSendTime* rembServer{ nullptr };
		RTC::TransportCongestionControlServer* tccServer{ nullptr };
		Timer* rtcpTimer{ nullptr };
		Timer* rembLimiterTimer{ nullptr };
		// Others.
		bool destroying{ false };
		struct RTC::RtpHeaderExtensionIds rtpHeaderExtensionIds;
		RTC::RtpListener rtpListener;
		RTC::SctpListener sctpListener;
		RTC::RateCalculator recvTransmission;
		RTC::RateCalculator sendTransmission;
		uint32_t initialAvailableOutgoingBitrate{ 600000 };
		uint32_t maxIncomingBitrate{ 0 };
	};

	/* Inline instance methods. */

	inline void Transport::DataReceived(size_t len)
	{
		this->recvTransmission.Update(len, DepLibUV::GetTime());
	}

	inline void Transport::DataSent(size_t len)
	{
		this->sendTransmission.Update(len, DepLibUV::GetTime());
	}
} // namespace RTC

#endif
