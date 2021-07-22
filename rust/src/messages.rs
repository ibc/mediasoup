use crate::active_speaker_observer::ActiveSpeakerObserverOptions;
use crate::audio_level_observer::AudioLevelObserverOptions;
use crate::consumer::{
    ConsumerDump, ConsumerId, ConsumerLayers, ConsumerScore, ConsumerStats, ConsumerTraceEventType,
    ConsumerType,
};
use crate::data_consumer::{DataConsumerDump, DataConsumerId, DataConsumerStat, DataConsumerType};
use crate::data_producer::{DataProducerDump, DataProducerId, DataProducerStat, DataProducerType};
use crate::data_structures::{
    DtlsParameters, DtlsRole, DtlsState, IceCandidate, IceParameters, IceRole, IceState, SctpState,
    TransportListenIp, TransportTuple,
};
use crate::direct_transport::DirectTransportOptions;
use crate::ortc::RtpMapping;
use crate::pipe_transport::PipeTransportOptions;
use crate::plain_transport::PlainTransportOptions;
use crate::producer::{
    ProducerDump, ProducerId, ProducerStat, ProducerTraceEventType, ProducerType,
};
use crate::router::{RouterDump, RouterId};
use crate::rtp_observer::RtpObserverId;
use crate::rtp_parameters::{MediaKind, RtpEncodingParameters, RtpParameters};
use crate::sctp_parameters::{NumSctpStreams, SctpParameters, SctpStreamParameters};
use crate::srtp_parameters::{SrtpCryptoSuite, SrtpParameters};
use crate::transport::{TransportId, TransportTraceEventType};
use crate::webrtc_transport::{TransportListenIps, WebRtcTransportOptions};
use crate::worker::{WorkerDump, WorkerUpdateSettings};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::IpAddr;
use std::num::NonZeroU16;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterInternal {
    pub(crate) router_id: RouterId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportInternal {
    pub(crate) router_id: RouterId,
    pub(crate) transport_id: TransportId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RtpObserverInternal {
    pub(crate) router_id: RouterId,
    pub(crate) rtp_observer_id: RtpObserverId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProducerInternal {
    pub(crate) router_id: RouterId,
    pub(crate) transport_id: TransportId,
    pub(crate) producer_id: ProducerId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsumerInternal {
    pub(crate) router_id: RouterId,
    pub(crate) transport_id: TransportId,
    pub(crate) consumer_id: ConsumerId,
    pub(crate) producer_id: ProducerId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataProducerInternal {
    pub(crate) router_id: RouterId,
    pub(crate) transport_id: TransportId,
    pub(crate) data_producer_id: DataProducerId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataConsumerInternal {
    pub(crate) router_id: RouterId,
    pub(crate) transport_id: TransportId,
    pub(crate) data_producer_id: DataProducerId,
    pub(crate) data_consumer_id: DataConsumerId,
}

pub(crate) trait Request: Debug + Serialize {
    type Response: DeserializeOwned;

    fn as_method(&self) -> &'static str;
}

pub(crate) trait Notification: Debug + Serialize {
    fn as_event(&self) -> &'static str;
}

macro_rules! request_response {
    (
        $method: literal,
        $request_struct_name: ident { $( $request_field_name: ident: $request_field_type: ty$(,)? )* },
        $existing_response_type: ty $(,)?
    ) => {
        #[derive(Debug, Serialize)]
        pub(crate) struct $request_struct_name {
            $( pub(crate) $request_field_name: $request_field_type, )*
        }

        impl Request for $request_struct_name {
            type Response = $existing_response_type;

            fn as_method(&self) -> &'static str {
                $method
            }
        }
    };
    // Call above macro with unit type as expected response
    (
        $method: literal,
        $request_struct_name: ident $request_struct_impl: tt $(,)?
    ) => {
        request_response!($method, $request_struct_name $request_struct_impl, ());
    };
    (
        $method: literal,
        $request_struct_name: ident { $( $request_field_name: ident: $request_field_type: ty$(,)? )* },
        $response_struct_name: ident { $( $response_field_name: ident: $response_field_type: ty$(,)? )* },
    ) => {
        #[derive(Debug, Serialize)]
        pub(crate) struct $request_struct_name {
            $( pub(crate) $request_field_name: $request_field_type, )*
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub(crate) struct $response_struct_name {
            $( pub(crate) $response_field_name: $response_field_type, )*
        }

        impl Request for $request_struct_name {
            type Response = $response_struct_name;

            fn as_method(&self) -> &'static str {
                $method
            }
        }
    };
}

macro_rules! request_response_generic {
    (
        $method: literal,
        $request_struct_name: ident { $( $request_field_name: ident: $request_field_type: ty$(,)? )* },
        $generic_response: ident,
    ) => {
        #[derive(Debug, Serialize)]
        pub(crate) struct $request_struct_name<$generic_response>
        where
            $generic_response: Debug + DeserializeOwned,
        {
            $( pub(crate) $request_field_name: $request_field_type, )*
            #[serde(skip)]
            pub(crate) phantom_data: PhantomData<$generic_response>,
        }

        impl<$generic_response: Debug + DeserializeOwned> Request for $request_struct_name<$generic_response> {
            type Response = $generic_response;

            fn as_method(&self) -> &'static str {
                $method
            }
        }
    };
}

request_response!("worker.close", WorkerCloseRequest {});

request_response!("worker.dump", WorkerDumpRequest {}, WorkerDump);

request_response!(
    "worker.updateSettings",
    WorkerUpdateSettingsRequest {
        data: WorkerUpdateSettings,
    },
);

request_response!(
    "worker.createRouter",
    WorkerCreateRouterRequest {
        internal: RouterInternal,
    },
);

request_response!(
    "router.close",
    RouterCloseRequest {
        internal: RouterInternal,
    },
);

request_response!(
    "router.dump",
    RouterDumpRequest {
        internal: RouterInternal,
    },
    RouterDump,
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreateDirectTransportData {
    pub(crate) direct: bool,
    pub(crate) max_message_size: usize,
}

impl RouterCreateDirectTransportData {
    pub(crate) fn from_options(direct_transport_options: &DirectTransportOptions) -> Self {
        Self {
            direct: true,
            max_message_size: direct_transport_options.max_message_size,
        }
    }
}

request_response!(
    "router.createDirectTransport",
    RouterCreateDirectTransportRequest {
        internal: TransportInternal,
        data: RouterCreateDirectTransportData,
    },
    RouterCreateDirectTransportResponse {},
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreateWebrtcTransportData {
    listen_ips: TransportListenIps,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    enable_udp: bool,
    enable_tcp: bool,
    prefer_udp: bool,
    prefer_tcp: bool,
    initial_available_outgoing_bitrate: u32,
    enable_sctp: bool,
    num_sctp_streams: NumSctpStreams,
    max_sctp_message_size: u32,
    sctp_send_buffer_size: u32,
    is_data_channel: bool,
}

impl RouterCreateWebrtcTransportData {
    pub(crate) fn from_options(webrtc_transport_options: &WebRtcTransportOptions) -> Self {
        Self {
            listen_ips: webrtc_transport_options.listen_ips.clone(),
            port: webrtc_transport_options.port,
            enable_udp: webrtc_transport_options.enable_udp,
            enable_tcp: webrtc_transport_options.enable_tcp,
            prefer_udp: webrtc_transport_options.prefer_udp,
            prefer_tcp: webrtc_transport_options.prefer_tcp,
            initial_available_outgoing_bitrate: webrtc_transport_options
                .initial_available_outgoing_bitrate,
            enable_sctp: webrtc_transport_options.enable_sctp,
            num_sctp_streams: webrtc_transport_options.num_sctp_streams,
            max_sctp_message_size: webrtc_transport_options.max_sctp_message_size,
            sctp_send_buffer_size: webrtc_transport_options.sctp_send_buffer_size,
            is_data_channel: true,
        }
    }
}

request_response!(
    "router.createWebRtcTransport",
    RouterCreateWebrtcTransportRequest {
        internal: TransportInternal,
        data: RouterCreateWebrtcTransportData,
    },
    WebRtcTransportData {
        ice_role: IceRole,
        ice_parameters: IceParameters,
        ice_candidates: Vec<IceCandidate>,
        ice_state: Mutex<IceState>,
        ice_selected_tuple: Mutex<Option<TransportTuple>>,
        dtls_parameters: Mutex<DtlsParameters>,
        dtls_state: Mutex<DtlsState>,
        dtls_remote_cert: Mutex<Option<String>>,
        sctp_parameters: Option<SctpParameters>,
        sctp_state: Mutex<Option<SctpState>>,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreatePlainTransportData {
    listen_ip: TransportListenIp,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    rtcp_mux: bool,
    comedia: bool,
    enable_sctp: bool,
    num_sctp_streams: NumSctpStreams,
    max_sctp_message_size: u32,
    sctp_send_buffer_size: u32,
    enable_srtp: bool,
    srtp_crypto_suite: SrtpCryptoSuite,
    is_data_channel: bool,
}

impl RouterCreatePlainTransportData {
    pub(crate) fn from_options(plain_transport_options: &PlainTransportOptions) -> Self {
        Self {
            listen_ip: plain_transport_options.listen_ip,
            port: plain_transport_options.port,
            rtcp_mux: plain_transport_options.rtcp_mux,
            comedia: plain_transport_options.comedia,
            enable_sctp: plain_transport_options.enable_sctp,
            num_sctp_streams: plain_transport_options.num_sctp_streams,
            max_sctp_message_size: plain_transport_options.max_sctp_message_size,
            sctp_send_buffer_size: plain_transport_options.sctp_send_buffer_size,
            enable_srtp: plain_transport_options.enable_srtp,
            srtp_crypto_suite: plain_transport_options.srtp_crypto_suite,
            is_data_channel: false,
        }
    }
}

request_response!(
    "router.createPlainTransport",
    RouterCreatePlainTransportRequest {
        internal: TransportInternal,
        data: RouterCreatePlainTransportData,
    },
    PlainTransportData {
        rtcp_mux: bool,
        comedia: bool,
        tuple: Mutex<TransportTuple>,
        rtcp_tuple: Mutex<Option<TransportTuple>>,
        sctp_parameters: Option<SctpParameters>,
        sctp_state: Mutex<Option<SctpState>>,
        srtp_parameters: Mutex<Option<SrtpParameters>>,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreatePipeTransportData {
    listen_ip: TransportListenIp,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    enable_sctp: bool,
    num_sctp_streams: NumSctpStreams,
    max_sctp_message_size: u32,
    sctp_send_buffer_size: u32,
    enable_rtx: bool,
    enable_srtp: bool,
    is_data_channel: bool,
}

impl RouterCreatePipeTransportData {
    pub(crate) fn from_options(pipe_transport_options: &PipeTransportOptions) -> Self {
        Self {
            listen_ip: pipe_transport_options.listen_ip,
            port: pipe_transport_options.port,
            enable_sctp: pipe_transport_options.enable_sctp,
            num_sctp_streams: pipe_transport_options.num_sctp_streams,
            max_sctp_message_size: pipe_transport_options.max_sctp_message_size,
            sctp_send_buffer_size: pipe_transport_options.sctp_send_buffer_size,
            enable_rtx: pipe_transport_options.enable_rtx,
            enable_srtp: pipe_transport_options.enable_srtp,
            is_data_channel: false,
        }
    }
}

request_response!(
    "router.createPipeTransport",
    RouterCreatePipeTransportRequest {
        internal: TransportInternal,
        data: RouterCreatePipeTransportData,
    },
    PipeTransportData {
        tuple: Mutex<TransportTuple>,
        sctp_parameters: Option<SctpParameters>,
        sctp_state: Mutex<Option<SctpState>>,
        rtx: bool,
        srtp_parameters: Mutex<Option<SrtpParameters>>,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreateAudioLevelObserverData {
    pub(crate) max_entries: NonZeroU16,
    pub(crate) threshold: i8,
    pub(crate) interval: u16,
}

impl RouterCreateAudioLevelObserverData {
    pub(crate) fn from_options(audio_level_observer_options: &AudioLevelObserverOptions) -> Self {
        Self {
            max_entries: audio_level_observer_options.max_entries,
            threshold: audio_level_observer_options.threshold,
            interval: audio_level_observer_options.interval,
        }
    }
}

request_response!(
    "router.createAudioLevelObserver",
    RouterCreateAudioLevelObserverRequest {
        internal: RtpObserverInternal,
        data: RouterCreateAudioLevelObserverData,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreateActiveSpeakerObserverData {
    pub(crate) interval: u16,
}

impl RouterCreateActiveSpeakerObserverData {
    pub(crate) fn from_options(
        active_speaker_observer_options: &ActiveSpeakerObserverOptions,
    ) -> Self {
        Self {
            interval: active_speaker_observer_options.interval,
        }
    }
}

request_response!(
    "router.createActiveSpeakerObserver",
    RouterCreateActiveSpeakerObserverRequest {
        internal: RtpObserverInternal,
        data: RouterCreateActiveSpeakerObserverData,
    },
);

request_response!(
    "transport.close",
    TransportCloseRequest {
        internal: TransportInternal,
    },
);

request_response_generic!(
    "transport.dump",
    TransportDumpRequest {
        internal: TransportInternal,
    },
    Dump,
);

request_response_generic!(
    "transport.getStats",
    TransportGetStatsRequest {
        internal: TransportInternal,
    },
    Stats,
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportConnectRequestWebRtcData {
    pub(crate) dtls_parameters: DtlsParameters,
}

request_response!(
    "transport.connect",
    TransportConnectRequestWebRtc {
        internal: TransportInternal,
        data: TransportConnectRequestWebRtcData,
    },
    TransportConnectResponseWebRtc {
        dtls_local_role: DtlsRole,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportConnectRequestPipeData {
    pub(crate) ip: IpAddr,
    pub(crate) port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) srtp_parameters: Option<SrtpParameters>,
}

request_response!(
    "transport.connect",
    TransportConnectRequestPipe {
        internal: TransportInternal,
        data: TransportConnectRequestPipeData,
    },
    TransportConnectResponsePipe {
        tuple: TransportTuple,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportConnectRequestPlainData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ip: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rtcp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) srtp_parameters: Option<SrtpParameters>,
}

request_response!(
    "transport.connect",
    TransportConnectRequestPlain {
        internal: TransportInternal,
        data: TransportConnectRequestPlainData,
    },
    TransportConnectResponsePlain {
        tuple: Option<TransportTuple>,
        rtcp_tuple: Option<TransportTuple>,
        srtp_parameters: Option<SrtpParameters>,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportSetMaxIncomingBitrateData {
    pub(crate) bitrate: u32,
}

request_response!(
    "transport.setMaxIncomingBitrate",
    TransportSetMaxIncomingBitrateRequest {
        internal: TransportInternal,
        data: TransportSetMaxIncomingBitrateData,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportSetMaxOutgoingBitrateData {
    pub(crate) bitrate: u32,
}

request_response!(
    "transport.setMaxOutgoingBitrate",
    TransportSetMaxOutgoingBitrateRequest {
        internal: TransportInternal,
        data: TransportSetMaxOutgoingBitrateData,
    },
);

request_response!(
    "transport.restartIce",
    TransportRestartIceRequest {
        internal: TransportInternal,
    },
    TransportRestartIceResponse {
        ice_parameters: IceParameters,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportProduceData {
    pub(crate) kind: MediaKind,
    pub(crate) rtp_parameters: RtpParameters,
    pub(crate) rtp_mapping: RtpMapping,
    pub(crate) key_frame_request_delay: u32,
    pub(crate) paused: bool,
}

request_response!(
    "transport.produce",
    TransportProduceRequest {
        internal: ProducerInternal,
        data: TransportProduceData,
    },
    TransportProduceResponse {
        r#type: ProducerType,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportConsumeData {
    pub(crate) kind: MediaKind,
    pub(crate) rtp_parameters: RtpParameters,
    pub(crate) r#type: ConsumerType,
    pub(crate) consumable_rtp_encodings: Vec<RtpEncodingParameters>,
    pub(crate) paused: bool,
    pub(crate) preferred_layers: Option<ConsumerLayers>,
}

request_response!(
    "transport.consume",
    TransportConsumeRequest {
        internal: ConsumerInternal,
        data: TransportConsumeData,
    },
    TransportConsumeResponse {
        paused: bool,
        producer_paused: bool,
        score: ConsumerScore,
        preferred_layers: Option<ConsumerLayers>,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportProduceDataData {
    pub(crate) r#type: DataProducerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sctp_stream_parameters: Option<SctpStreamParameters>,
    pub(crate) label: String,
    pub(crate) protocol: String,
}

request_response!(
    "transport.produceData",
    TransportProduceDataRequest {
        internal: DataProducerInternal,
        data: TransportProduceDataData,
    },
    TransportProduceDataResponse {
        r#type: DataProducerType,
        sctp_stream_parameters: Option<SctpStreamParameters>,
        label: String,
        protocol: String,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportConsumeDataData {
    pub(crate) r#type: DataConsumerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sctp_stream_parameters: Option<SctpStreamParameters>,
    pub(crate) label: String,
    pub(crate) protocol: String,
}

request_response!(
    "transport.consumeData",
    TransportConsumeDataRequest {
        internal: DataConsumerInternal,
        data: TransportConsumeDataData,
    },
    TransportConsumeDataResponse {
        r#type: DataConsumerType,
        sctp_stream_parameters: Option<SctpStreamParameters>,
        label: String,
        protocol: String,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportEnableTraceEventData {
    pub(crate) types: Vec<TransportTraceEventType>,
}

request_response!(
    "transport.enableTraceEvent",
    TransportEnableTraceEventRequest {
        internal: TransportInternal,
        data: TransportEnableTraceEventData,
    },
);

#[derive(Debug, Serialize)]
pub(crate) struct TransportSendRtcpNotification {
    pub(crate) internal: TransportInternal,
}

impl Notification for TransportSendRtcpNotification {
    fn as_event(&self) -> &'static str {
        "transport.sendRtcp"
    }
}

request_response!(
    "producer.close",
    ProducerCloseRequest {
        internal: ProducerInternal,
    },
);

request_response!(
    "producer.dump",
    ProducerDumpRequest {
        internal: ProducerInternal,
    },
    ProducerDump
);

request_response!(
    "producer.getStats",
    ProducerGetStatsRequest {
        internal: ProducerInternal,
    },
    Vec<ProducerStat>,
);

request_response!(
    "producer.pause",
    ProducerPauseRequest {
        internal: ProducerInternal,
    },
);

request_response!(
    "producer.resume",
    ProducerResumeRequest {
        internal: ProducerInternal,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProducerEnableTraceEventData {
    pub(crate) types: Vec<ProducerTraceEventType>,
}

request_response!(
    "producer.enableTraceEvent",
    ProducerEnableTraceEventRequest {
        internal: ProducerInternal,
        data: ProducerEnableTraceEventData,
    },
);

#[derive(Debug, Serialize)]
pub(crate) struct ProducerSendNotification {
    pub(crate) internal: ProducerInternal,
}

impl Notification for ProducerSendNotification {
    fn as_event(&self) -> &'static str {
        "producer.send"
    }
}

request_response!(
    "consumer.close",
    ConsumerCloseRequest {
        internal: ConsumerInternal,
    },
);

request_response!(
    "consumer.dump",
    ConsumerDumpRequest {
        internal: ConsumerInternal,
    },
    ConsumerDump,
);

request_response!(
    "consumer.getStats",
    ConsumerGetStatsRequest {
        internal: ConsumerInternal,
    },
    ConsumerStats,
);

request_response!(
    "consumer.pause",
    ConsumerPauseRequest {
        internal: ConsumerInternal,
    },
);

request_response!(
    "consumer.resume",
    ConsumerResumeRequest {
        internal: ConsumerInternal,
    },
);

request_response!(
    "consumer.setPreferredLayers",
    ConsumerSetPreferredLayersRequest {
        internal: ConsumerInternal,
        data: ConsumerLayers,
    },
    Option<ConsumerLayers>,
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsumerSetPriorityData {
    pub(crate) priority: u8,
}

request_response!(
    "consumer.setPriority",
    ConsumerSetPriorityRequest {
        internal: ConsumerInternal,
        data: ConsumerSetPriorityData,
    },
    ConsumerSetPriorityResponse { priority: u8 },
);

request_response!(
    "consumer.requestKeyFrame",
    ConsumerRequestKeyFrameRequest {
        internal: ConsumerInternal,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsumerEnableTraceEventData {
    pub(crate) types: Vec<ConsumerTraceEventType>,
}

request_response!(
    "producer.enableTraceEvent",
    ConsumerEnableTraceEventRequest {
        internal: ConsumerInternal,
        data: ConsumerEnableTraceEventData,
    },
);

request_response!(
    "dataProducer.close",
    DataProducerCloseRequest {
        internal: DataProducerInternal,
    },
);

request_response!(
    "dataProducer.dump",
    DataProducerDumpRequest {
        internal: DataProducerInternal,
    },
    DataProducerDump,
);

request_response!(
    "dataProducer.getStats",
    DataProducerGetStatsRequest {
        internal: DataProducerInternal,
    },
    Vec<DataProducerStat>,
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataProducerSendData {
    pub(crate) ppid: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct DataProducerSendNotification {
    pub(crate) internal: DataProducerInternal,
    pub(crate) data: DataProducerSendData,
}

impl Notification for DataProducerSendNotification {
    fn as_event(&self) -> &'static str {
        "dataProducer.send"
    }
}

request_response!(
    "dataConsumer.close",
    DataConsumerCloseRequest {
        internal: DataConsumerInternal
    },
);

request_response!(
    "dataConsumer.dump",
    DataConsumerDumpRequest {
        internal: DataConsumerInternal,
    },
    DataConsumerDump,
);

request_response!(
    "dataConsumer.getStats",
    DataConsumerGetStatsRequest {
        internal: DataConsumerInternal,
    },
    Vec<DataConsumerStat>,
);

request_response!(
    "dataConsumer.getBufferedAmount",
    DataConsumerGetBufferedAmountRequest {
        internal: DataConsumerInternal,
    },
    DataConsumerGetBufferedAmountResponse {
        buffered_amount: u32,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataConsumerSetBufferedAmountLowThresholdData {
    pub(crate) threshold: u32,
}

request_response!(
    "dataConsumer.setBufferedAmountLowThreshold",
    DataConsumerSetBufferedAmountLowThresholdRequest {
        internal: DataConsumerInternal,
        data: DataConsumerSetBufferedAmountLowThresholdData,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataConsumerSendRequestData {
    pub(crate) ppid: u32,
}

request_response!(
    "dataConsumer.send",
    DataConsumerSendRequest {
        internal: DataConsumerInternal,
        data: DataConsumerSendRequestData,
    },
);

request_response!(
    "rtpObserver.close",
    RtpObserverCloseRequest {
        internal: RtpObserverInternal,
    },
);

request_response!(
    "rtpObserver.pause",
    RtpObserverPauseRequest {
        internal: RtpObserverInternal,
    },
);

request_response!(
    "rtpObserver.resume",
    RtpObserverResumeRequest {
        internal: RtpObserverInternal,
    },
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RtpObserverAddRemoveProducerRequestData {
    pub(crate) producer_id: ProducerId,
}

request_response!(
    "rtpObserver.addProducer",
    RtpObserverAddProducerRequest {
        internal: RtpObserverInternal,
        data: RtpObserverAddRemoveProducerRequestData,
    },
);

request_response!(
    "rtpObserver.removeProducer",
    RtpObserverRemoveProducerRequest {
        internal: RtpObserverInternal,
        data: RtpObserverAddRemoveProducerRequestData,
    },
);
