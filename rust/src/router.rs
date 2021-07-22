//! A router enables injection, selection and forwarding of media streams through [`Transport`]
//! instances created on it.
//!
//! Developers may think of a mediasoup router as if it were a "multi-party conference room",
//! although mediasoup is much more low level than that and doesn't constrain itself to specific
//! high level use cases (for instance, a "multi-party conference room" could involve various
//! mediasoup routers, even in different physicals hosts).

pub(super) mod active_speaker_observer;
pub(super) mod audio_level_observer;
pub(super) mod consumer;
pub(super) mod data_consumer;
pub(super) mod data_producer;
pub(super) mod direct_transport;
pub(super) mod pipe_transport;
pub(super) mod plain_transport;
pub(super) mod producer;
pub(super) mod rtp_observer;
#[cfg(test)]
mod tests;
pub(super) mod transport;
pub(super) mod webrtc_transport;

use crate::active_speaker_observer::{ActiveSpeakerObserver, ActiveSpeakerObserverOptions};
use crate::audio_level_observer::{AudioLevelObserver, AudioLevelObserverOptions};
use crate::consumer::{Consumer, ConsumerId, ConsumerOptions};
use crate::data_consumer::{DataConsumer, DataConsumerId, DataConsumerOptions};
use crate::data_producer::{
    DataProducer, DataProducerId, DataProducerOptions, NonClosingDataProducer, WeakDataProducer,
};
use crate::data_structures::{AppData, TransportListenIp};
use crate::direct_transport::{DirectTransport, DirectTransportOptions};
use crate::messages::{
    RouterCloseRequest, RouterCreateActiveSpeakerObserverData,
    RouterCreateActiveSpeakerObserverRequest, RouterCreateAudioLevelObserverData,
    RouterCreateAudioLevelObserverRequest, RouterCreateDirectTransportData,
    RouterCreateDirectTransportRequest, RouterCreatePipeTransportData,
    RouterCreatePipeTransportRequest, RouterCreatePlainTransportData,
    RouterCreatePlainTransportRequest, RouterCreateWebrtcTransportData,
    RouterCreateWebrtcTransportRequest, RouterDumpRequest, RouterInternal, RtpObserverInternal,
    TransportInternal,
};
use crate::pipe_transport::{
    PipeTransport, PipeTransportOptions, PipeTransportRemoteParameters, WeakPipeTransport,
};
use crate::plain_transport::{PlainTransport, PlainTransportOptions};
use crate::producer::{PipedProducer, Producer, ProducerId, ProducerOptions, WeakProducer};
use crate::rtp_observer::{RtpObserver, RtpObserverId};
use crate::rtp_parameters::{RtpCapabilities, RtpCapabilitiesFinalized, RtpCodecCapability};
use crate::sctp_parameters::NumSctpStreams;
use crate::transport::{
    ConsumeDataError, ConsumeError, ProduceDataError, ProduceError, Transport, TransportGeneric,
    TransportId,
};
use crate::webrtc_transport::{WebRtcTransport, WebRtcTransportOptions};
use crate::worker::{Channel, PayloadChannel, RequestError, Worker};
use crate::{ortc, uuid_based_wrapper_type};
use async_executor::Executor;
use async_lock::Mutex as AsyncMutex;
use event_listener_primitives::{Bag, BagOnce, HandlerId};
use futures_lite::future;
use log::{debug, error};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use thiserror::Error;

uuid_based_wrapper_type!(
    /// [`Router`] identifier.
    RouterId
);

/// [`Router`] options.
///
/// # Notes on usage
///
/// * Feature codecs such as `RTX` MUST NOT be placed into the mediaCodecs list.
/// * If `preferred_payload_type` is given in a [`RtpCodecCapability`] (although it's unnecessary)
///   it's extremely recommended to use a value in the 96-127 range.
#[derive(Debug)]
#[non_exhaustive]
pub struct RouterOptions {
    /// Router media codecs.
    pub media_codecs: Vec<RtpCodecCapability>,
    /// Custom application data.
    pub app_data: AppData,
}

impl RouterOptions {
    /// Create router options with given list of declared media codecs.
    #[must_use]
    pub fn new(media_codecs: Vec<RtpCodecCapability>) -> Self {
        Self {
            media_codecs,
            app_data: AppData::default(),
        }
    }
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self::new(vec![])
    }
}

/// Options used for piping media or data producer to into another router on the same host.
///
/// # Notes on usage
/// * SCTP arguments will only apply the first time the underlying transports are created.
#[derive(Debug)]
pub struct PipeToRouterOptions {
    /// Target Router instance.
    pub router: Router,
    /// IP used in the PipeTransport pair.
    ///
    /// Default `127.0.0.1`.
    listen_ip: TransportListenIp,
    /// Create a SCTP association.
    ///
    /// Default `true`.
    pub enable_sctp: bool,
    /// SCTP streams number.
    pub num_sctp_streams: NumSctpStreams,
    /// Enable RTX and NACK for RTP retransmission.
    ///
    /// Default `false`.
    pub enable_rtx: bool,
    /// Enable SRTP.
    ///
    /// Default `false`.
    pub enable_srtp: bool,
}

impl PipeToRouterOptions {
    /// Crate pipe options for piping into given local router.
    #[must_use]
    pub fn new(router: Router) -> Self {
        Self {
            router,
            listen_ip: TransportListenIp {
                ip: "127.0.0.1".parse().unwrap(),
                announced_ip: None,
            },
            enable_sctp: true,
            num_sctp_streams: NumSctpStreams::default(),
            enable_rtx: false,
            enable_srtp: false,
        }
    }
}

/// Container for pipe consumer and pipe producer pair.
///
/// # Notes on usage
/// Pipe consumer and Pipe producer will not be closed on drop, to control this manually get pipe
/// producer out of non-closing variant with [`PipedProducer::into_inner()`] call,
/// otherwise pipe consumer and pipe producer lifetime will be tied to source producer lifetime.
///
/// Pipe consumer is always tied to the lifetime of pipe producer.
#[derive(Debug)]
pub struct PipeProducerToRouterPair {
    /// The Consumer created in the current Router.
    pub pipe_consumer: Consumer,
    /// The Producer created in the target Router, get regular instance with
    /// [`PipedProducer::into_inner()`] call.
    pub pipe_producer: PipedProducer,
}

/// Error that caused [`Router::pipe_producer_to_router()`] to fail.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum PipeProducerToRouterError {
    /// Destination router must be different
    #[error("Destination router must be different")]
    SameRouter,
    /// Producer with specified id not found
    #[error("Producer with id \"{0}\" not found")]
    ProducerNotFound(ProducerId),
    /// Failed to create or connect Pipe transport
    #[error("Failed to create or connect Pipe transport: \"{0}\"")]
    TransportFailed(RequestError),
    /// Failed to consume
    #[error("Failed to consume: \"{0}\"")]
    ConsumeFailed(ConsumeError),
    /// Failed to produce
    #[error("Failed to produce: \"{0}\"")]
    ProduceFailed(ProduceError),
}

impl From<RequestError> for PipeProducerToRouterError {
    fn from(error: RequestError) -> Self {
        PipeProducerToRouterError::TransportFailed(error)
    }
}

impl From<ConsumeError> for PipeProducerToRouterError {
    fn from(error: ConsumeError) -> Self {
        PipeProducerToRouterError::ConsumeFailed(error)
    }
}

impl From<ProduceError> for PipeProducerToRouterError {
    fn from(error: ProduceError) -> Self {
        PipeProducerToRouterError::ProduceFailed(error)
    }
}

/// Container for pipe data consumer and pipe data producer pair.
///
/// # Notes on usage
/// Pipe data consumer and pipe data producer will not be closed on drop, to control this manually
/// get pipe data producer out of non-closing variant with [`NonClosingDataProducer::into_inner()`]
/// call, otherwise pipe data consumer and pipe data producer lifetime will be tied to source data
/// producer lifetime.
///
/// Pipe data consumer is always tied to the lifetime of pipe data producer.
#[derive(Debug)]
pub struct PipeDataProducerToRouterPair {
    /// The DataConsumer created in the current Router.
    pub pipe_data_consumer: DataConsumer,
    /// The DataProducer created in the target Router, get regular instance with
    /// [`NonClosingDataProducer::into_inner()`] call.
    pub pipe_data_producer: NonClosingDataProducer,
}

/// Error that caused [`Router::pipe_data_producer_to_router()`] to fail.
#[derive(Debug, Error)]
pub enum PipeDataProducerToRouterError {
    /// Destination router must be different
    #[error("Destination router must be different")]
    SameRouter,
    /// Data producer with specified id not found
    #[error("Data producer with id \"{0}\" not found")]
    DataProducerNotFound(DataProducerId),
    /// Failed to create or connect Pipe transport
    #[error("Failed to create or connect Pipe transport: \"{0}\"")]
    TransportFailed(RequestError),
    /// Failed to consume
    #[error("Failed to consume: \"{0}\"")]
    ConsumeFailed(ConsumeDataError),
    /// Failed to produce
    #[error("Failed to produce: \"{0}\"")]
    ProduceFailed(ProduceDataError),
}

impl From<RequestError> for PipeDataProducerToRouterError {
    fn from(error: RequestError) -> Self {
        PipeDataProducerToRouterError::TransportFailed(error)
    }
}

impl From<ConsumeDataError> for PipeDataProducerToRouterError {
    fn from(error: ConsumeDataError) -> Self {
        PipeDataProducerToRouterError::ConsumeFailed(error)
    }
}

impl From<ProduceDataError> for PipeDataProducerToRouterError {
    fn from(error: ProduceDataError) -> Self {
        PipeDataProducerToRouterError::ProduceFailed(error)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[doc(hidden)]
#[non_exhaustive]
pub struct RouterDump {
    pub id: RouterId,
    pub map_consumer_id_producer_id: HashMap<ConsumerId, ProducerId>,
    pub map_data_consumer_id_data_producer_id: HashMap<DataConsumerId, DataProducerId>,
    pub map_data_producer_id_data_consumer_ids: HashMap<DataProducerId, HashSet<DataConsumerId>>,
    pub map_producer_id_consumer_ids: HashMap<ProducerId, HashSet<ConsumerId>>,
    pub map_producer_id_observer_ids: HashMap<ProducerId, HashSet<RtpObserverId>>,
    pub rtp_observer_ids: HashSet<RtpObserverId>,
    pub transport_ids: HashSet<TransportId>,
}

/// New transport that was just created.
#[derive(Debug)]
pub enum NewTransport<'a> {
    /// Direct transport
    Direct(&'a DirectTransport),
    /// Pipe transport
    Pipe(&'a PipeTransport),
    /// Plain transport
    Plain(&'a PlainTransport),
    /// WebRtc transport
    WebRtc(&'a WebRtcTransport),
}

impl<'a> Deref for NewTransport<'a> {
    type Target = dyn Transport;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Direct(transport) => *transport as &Self::Target,
            Self::Pipe(transport) => *transport as &Self::Target,
            Self::Plain(transport) => *transport as &Self::Target,
            Self::WebRtc(transport) => *transport as &Self::Target,
        }
    }
}

/// New RTP observer that was just created.
#[derive(Debug)]
pub enum NewRtpObserver<'a> {
    /// Audio level observer
    AudioLevel(&'a AudioLevelObserver),
    /// Active speaker observer
    ActiveSpeaker(&'a ActiveSpeakerObserver),
}

impl<'a> Deref for NewRtpObserver<'a> {
    type Target = dyn RtpObserver;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::AudioLevel(observer) => *observer as &Self::Target,
            Self::ActiveSpeaker(observer) => *observer as &Self::Target,
        }
    }
}

struct PipeTransportPair {
    local: PipeTransport,
    remote: PipeTransport,
}

impl PipeTransportPair {
    fn downgrade(&self) -> WeakPipeTransportPair {
        WeakPipeTransportPair {
            local: self.local.downgrade(),
            remote: self.remote.downgrade(),
        }
    }
}

#[derive(Debug)]
struct WeakPipeTransportPair {
    local: WeakPipeTransport,
    remote: WeakPipeTransport,
}

impl WeakPipeTransportPair {
    fn upgrade(&self) -> Option<PipeTransportPair> {
        let local = self.local.upgrade()?;
        let remote = self.remote.upgrade()?;
        Some(PipeTransportPair { local, remote })
    }
}

#[derive(Default)]
struct Handlers {
    new_transport: Bag<Box<dyn Fn(NewTransport<'_>) + Send + Sync>>,
    new_rtp_observer: Bag<Box<dyn Fn(NewRtpObserver<'_>) + Send + Sync>>,
    worker_close: BagOnce<Box<dyn FnOnce() + Send>>,
    close: BagOnce<Box<dyn FnOnce() + Send>>,
}

struct Inner {
    id: RouterId,
    executor: Arc<Executor<'static>>,
    rtp_capabilities: RtpCapabilitiesFinalized,
    channel: Channel,
    payload_channel: PayloadChannel,
    handlers: Arc<Handlers>,
    app_data: AppData,
    producers: Arc<RwLock<HashMap<ProducerId, WeakProducer>>>,
    data_producers: Arc<RwLock<HashMap<DataProducerId, WeakDataProducer>>>,
    #[allow(clippy::type_complexity)]
    mapped_pipe_transports:
        Arc<Mutex<HashMap<RouterId, Arc<AsyncMutex<Option<WeakPipeTransportPair>>>>>>,
    // Make sure worker is not dropped until this router is not dropped
    worker: Option<Worker>,
    closed: AtomicBool,
    _on_worker_close_handler: Mutex<HandlerId>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        debug!("drop()");

        self.close();
    }
}

impl Inner {
    fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.handlers.close.call_simple();

            {
                let channel = self.channel.clone();
                let request = RouterCloseRequest {
                    internal: RouterInternal { router_id: self.id },
                };
                let worker = self.worker.clone();
                self.executor
                    .spawn(async move {
                        if let Err(error) = channel.request(request).await {
                            error!("router closing failed on drop: {}", error);
                        }

                        drop(worker);
                    })
                    .detach();
            }
        }
    }
}

/// A router enables injection, selection and forwarding of media streams through [`Transport`]
/// instances created on it.
///
/// Developers may think of a mediasoup router as if it were a "multi-party conference room",
/// although mediasoup is much more low level than that and doesn't constrain itself to specific
/// high level use cases (for instance, a "multi-party conference room" could involve various
/// mediasoup routers, even in different physicals hosts).
#[derive(Clone)]
#[must_use = "Router will be closed on drop, make sure to keep it around for as long as needed"]
pub struct Router {
    inner: Arc<Inner>,
}

impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("id", &self.inner.id)
            .field("rtp_capabilities", &self.inner.rtp_capabilities)
            .field("producers", &self.inner.producers)
            .field("data_producers", &self.inner.data_producers)
            .field("mapped_pipe_transports", &self.inner.mapped_pipe_transports)
            .field("worker", &self.inner.worker)
            .field("closed", &self.inner.closed)
            .finish()
    }
}

impl Router {
    pub(super) fn new(
        id: RouterId,
        executor: Arc<Executor<'static>>,
        channel: Channel,
        payload_channel: PayloadChannel,
        rtp_capabilities: RtpCapabilitiesFinalized,
        app_data: AppData,
        worker: Worker,
    ) -> Self {
        debug!("new()");

        let producers = Arc::<RwLock<HashMap<ProducerId, WeakProducer>>>::default();
        let data_producers = Arc::<RwLock<HashMap<DataProducerId, WeakDataProducer>>>::default();
        let mapped_pipe_transports = Arc::<
            Mutex<HashMap<RouterId, Arc<AsyncMutex<Option<WeakPipeTransportPair>>>>>,
        >::default();
        let handlers = Arc::<Handlers>::default();
        let inner_weak = Arc::<Mutex<Option<Weak<Inner>>>>::default();
        let on_worker_close_handler = worker.on_close({
            let inner_weak = Arc::clone(&inner_weak);

            move || {
                if let Some(inner) = inner_weak.lock().as_ref().and_then(Weak::upgrade) {
                    inner.handlers.worker_close.call_simple();
                    if !inner.closed.swap(true, Ordering::SeqCst) {
                        inner.handlers.close.call_simple();
                    }
                }
            }
        });
        let inner = Arc::new(Inner {
            id,
            executor,
            rtp_capabilities,
            channel,
            payload_channel,
            handlers,
            producers,
            data_producers,
            mapped_pipe_transports,
            app_data,
            worker: Some(worker),
            closed: AtomicBool::new(false),
            _on_worker_close_handler: Mutex::new(on_worker_close_handler),
        });

        inner_weak.lock().replace(Arc::downgrade(&inner));

        Self { inner }
    }

    /// Router id.
    #[must_use]
    pub fn id(&self) -> RouterId {
        self.inner.id
    }

    /// Custom application data.
    #[must_use]
    pub fn app_data(&self) -> &AppData {
        &self.inner.app_data
    }

    /// Whether router is closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::SeqCst)
    }

    /// RTP capabilities of the router. These capabilities are typically needed by mediasoup clients
    /// to compute their sending RTP parameters.
    ///
    /// # Notes on usage
    /// * Check the [RTP Parameters and Capabilities](https://mediasoup.org/documentation/v3/mediasoup/rtp-parameters-and-capabilities/)
    ///   section for more details.
    /// * See also how to [filter these RTP capabilities](https://mediasoup.org/documentation/v3/tricks/#rtp-capabilities-filtering)
    ///   before using them into a client.
    #[must_use]
    pub fn rtp_capabilities(&self) -> &RtpCapabilitiesFinalized {
        &self.inner.rtp_capabilities
    }

    /// Dump Router.
    #[doc(hidden)]
    pub async fn dump(&self) -> Result<RouterDump, RequestError> {
        debug!("dump()");

        self.inner
            .channel
            .request(RouterDumpRequest {
                internal: RouterInternal {
                    router_id: self.inner.id,
                },
            })
            .await
    }

    /// Create a [`DirectTransport`].
    ///
    /// Router will be kept alive as long as at least one transport instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::direct_transport::DirectTransportOptions;
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let transport = router.create_direct_transport(DirectTransportOptions::default()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_direct_transport(
        &self,
        direct_transport_options: DirectTransportOptions,
    ) -> Result<DirectTransport, RequestError> {
        debug!("create_direct_transport()");

        let transport_id = TransportId::new();

        let _buffer_guard = self.inner.channel.buffer_messages_for(transport_id.into());

        self.inner
            .channel
            .request(RouterCreateDirectTransportRequest {
                internal: TransportInternal {
                    router_id: self.inner.id,
                    transport_id,
                },
                data: RouterCreateDirectTransportData::from_options(&direct_transport_options),
            })
            .await?;

        let transport = DirectTransport::new(
            transport_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            self.inner.payload_channel.clone(),
            direct_transport_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_transport.call(|callback| {
            callback(NewTransport::Direct(&transport));
        });

        self.after_transport_creation(&transport);

        Ok(transport)
    }

    /// Create a [`WebRtcTransport`].
    ///
    /// Router will be kept alive as long as at least one transport instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::data_structures::TransportListenIp;
    /// use mediasoup::webrtc_transport::{TransportListenIps, WebRtcTransportOptions};
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let transport = router
    ///     .create_webrtc_transport(WebRtcTransportOptions::new(TransportListenIps::new(
    ///         TransportListenIp {
    ///             ip: "127.0.0.1".parse().unwrap(),
    ///             announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///         },
    ///     )))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_webrtc_transport(
        &self,
        webrtc_transport_options: WebRtcTransportOptions,
    ) -> Result<WebRtcTransport, RequestError> {
        debug!("create_webrtc_transport()");

        let transport_id = TransportId::new();

        let _buffer_guard = self.inner.channel.buffer_messages_for(transport_id.into());

        let data = self
            .inner
            .channel
            .request(RouterCreateWebrtcTransportRequest {
                internal: TransportInternal {
                    router_id: self.inner.id,
                    transport_id,
                },
                data: RouterCreateWebrtcTransportData::from_options(&webrtc_transport_options),
            })
            .await?;

        let transport = WebRtcTransport::new(
            transport_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            self.inner.payload_channel.clone(),
            data,
            webrtc_transport_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_transport.call(|callback| {
            callback(NewTransport::WebRtc(&transport));
        });

        self.after_transport_creation(&transport);

        Ok(transport)
    }

    /// Create a [`PipeTransport`].
    ///
    /// Router will be kept alive as long as at least one transport instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::data_structures::TransportListenIp;
    /// use mediasoup::pipe_transport::PipeTransportOptions;
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let transport = router
    ///     .create_pipe_transport(PipeTransportOptions::new(TransportListenIp {
    ///         ip: "127.0.0.1".parse().unwrap(),
    ///         announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///     }))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_pipe_transport(
        &self,
        pipe_transport_options: PipeTransportOptions,
    ) -> Result<PipeTransport, RequestError> {
        debug!("create_pipe_transport()");

        let transport_id = TransportId::new();

        let _buffer_guard = self.inner.channel.buffer_messages_for(transport_id.into());

        let data = self
            .inner
            .channel
            .request(RouterCreatePipeTransportRequest {
                internal: TransportInternal {
                    router_id: self.inner.id,
                    transport_id,
                },
                data: RouterCreatePipeTransportData::from_options(&pipe_transport_options),
            })
            .await?;

        let transport = PipeTransport::new(
            transport_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            self.inner.payload_channel.clone(),
            data,
            pipe_transport_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_transport.call(|callback| {
            callback(NewTransport::Pipe(&transport));
        });

        self.after_transport_creation(&transport);

        Ok(transport)
    }

    /// Create a [`PlainTransport`].
    ///
    /// Router will be kept alive as long as at least one transport instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::data_structures::TransportListenIp;
    /// use mediasoup::plain_transport::PlainTransportOptions;
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let transport = router
    ///     .create_plain_transport(PlainTransportOptions::new(TransportListenIp {
    ///         ip: "127.0.0.1".parse().unwrap(),
    ///         announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///     }))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_plain_transport(
        &self,
        plain_transport_options: PlainTransportOptions,
    ) -> Result<PlainTransport, RequestError> {
        debug!("create_plain_transport()");

        let transport_id = TransportId::new();

        let _buffer_guard = self.inner.channel.buffer_messages_for(transport_id.into());

        let data = self
            .inner
            .channel
            .request(RouterCreatePlainTransportRequest {
                internal: TransportInternal {
                    router_id: self.inner.id,
                    transport_id,
                },
                data: RouterCreatePlainTransportData::from_options(&plain_transport_options),
            })
            .await?;

        let transport = PlainTransport::new(
            transport_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            self.inner.payload_channel.clone(),
            data,
            plain_transport_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_transport.call(|callback| {
            callback(NewTransport::Plain(&transport));
        });

        self.after_transport_creation(&transport);

        Ok(transport)
    }

    /// Create an [`AudioLevelObserver`].
    ///
    /// Router will be kept alive as long as at least one observer instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::audio_level_observer::AudioLevelObserverOptions;
    /// use std::num::NonZeroU16;
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let observer = router
    ///     .create_audio_level_observer({
    ///         let mut options = AudioLevelObserverOptions::default();
    ///         options.max_entries = NonZeroU16::new(1).unwrap();
    ///         options.threshold = -70;
    ///         options.interval = 2000;
    ///         options
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_audio_level_observer(
        &self,
        audio_level_observer_options: AudioLevelObserverOptions,
    ) -> Result<AudioLevelObserver, RequestError> {
        debug!("create_audio_level_observer()");

        let rtp_observer_id = RtpObserverId::new();

        let _buffer_guard = self
            .inner
            .channel
            .buffer_messages_for(rtp_observer_id.into());

        self.inner
            .channel
            .request(RouterCreateAudioLevelObserverRequest {
                internal: RtpObserverInternal {
                    router_id: self.inner.id,
                    rtp_observer_id,
                },
                data: RouterCreateAudioLevelObserverData::from_options(
                    &audio_level_observer_options,
                ),
            })
            .await?;

        let audio_level_observer = AudioLevelObserver::new(
            rtp_observer_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            audio_level_observer_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_rtp_observer.call(|callback| {
            callback(NewRtpObserver::AudioLevel(&audio_level_observer));
        });

        Ok(audio_level_observer)
    }

    /// Create an [`ActiveSpeakerObserver`].
    ///
    /// Router will be kept alive as long as at least one observer instance is alive.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::active_speaker_observer::ActiveSpeakerObserverOptions;
    ///
    /// # async fn f(router: mediasoup::router::Router) -> Result<(), Box<dyn std::error::Error>> {
    /// let observer = router
    ///     .create_active_speaker_observer({
    ///         let mut options = ActiveSpeakerObserverOptions::default();
    ///         options.interval = 300;
    ///         options
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_active_speaker_observer(
        &self,
        active_speaker_observer_options: ActiveSpeakerObserverOptions,
    ) -> Result<ActiveSpeakerObserver, RequestError> {
        debug!("create_active_speaker_observer()");

        let rtp_observer_id = RtpObserverId::new();

        let _buffer_guard = self
            .inner
            .channel
            .buffer_messages_for(rtp_observer_id.into());

        self.inner
            .channel
            .request(RouterCreateActiveSpeakerObserverRequest {
                internal: RtpObserverInternal {
                    router_id: self.inner.id,
                    rtp_observer_id,
                },
                data: RouterCreateActiveSpeakerObserverData::from_options(
                    &active_speaker_observer_options,
                ),
            })
            .await?;

        let active_speaker_observer = ActiveSpeakerObserver::new(
            rtp_observer_id,
            Arc::clone(&self.inner.executor),
            self.inner.channel.clone(),
            active_speaker_observer_options.app_data,
            self.clone(),
        );

        self.inner.handlers.new_rtp_observer.call(|callback| {
            callback(NewRtpObserver::ActiveSpeaker(&active_speaker_observer));
        });

        Ok(active_speaker_observer)
    }

    /// Pipes [`Producer`] with the given `producer_id` into another [`Router`] on same host.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::consumer::ConsumerOptions;
    /// use mediasoup::data_structures::TransportListenIp;
    /// use mediasoup::producer::ProducerOptions;
    /// use mediasoup::router::{PipeToRouterOptions, RouterOptions};
    /// use mediasoup::rtp_parameters::{
    ///    MediaKind, MimeTypeAudio, RtcpParameters, RtpCapabilities, RtpCodecCapability,
    ///    RtpCodecParameters, RtpCodecParametersParameters, RtpParameters,
    /// };
    /// use mediasoup::transport::Transport;
    /// use mediasoup::webrtc_transport::{TransportListenIps, WebRtcTransportOptions};
    /// use mediasoup::worker::WorkerSettings;
    /// use std::num::{NonZeroU32, NonZeroU8};
    ///
    /// # async fn f(
    /// #     worker_manager: mediasoup::worker_manager::WorkerManager,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// // Have two workers.
    /// let worker1 = worker_manager.create_worker(WorkerSettings::default()).await?;
    /// let worker2 = worker_manager.create_worker(WorkerSettings::default()).await?;
    ///
    /// // Create a router in each worker.
    /// let media_codecs = vec![
    ///     RtpCodecCapability::Audio {
    ///         mime_type: MimeTypeAudio::Opus,
    ///         preferred_payload_type: None,
    ///         clock_rate: NonZeroU32::new(48000).unwrap(),
    ///         channels: NonZeroU8::new(2).unwrap(),
    ///         parameters: RtpCodecParametersParameters::from([
    ///             ("useinbandfec", 1_u32.into()),
    ///         ]),
    ///         rtcp_feedback: vec![],
    ///     },
    /// ];
    /// let router1 = worker1.create_router(RouterOptions::new(media_codecs.clone())).await?;
    /// let router2 = worker2.create_router(RouterOptions::new(media_codecs)).await?;
    ///
    /// // Produce in router1.
    /// let transport1 = router1
    ///     .create_webrtc_transport(WebRtcTransportOptions::new(TransportListenIps::new(
    ///         TransportListenIp {
    ///             ip: "127.0.0.1".parse().unwrap(),
    ///             announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///         },
    ///     )))
    ///     .await?;
    /// let producer1 = transport1
    ///     .produce(ProducerOptions::new(
    ///         MediaKind::Audio,
    ///         RtpParameters {
    ///             mid: Some("AUDIO".to_string()),
    ///             codecs: vec![RtpCodecParameters::Audio {
    ///                 mime_type: MimeTypeAudio::Opus,
    ///                 payload_type: 0,
    ///                 clock_rate: NonZeroU32::new(48000).unwrap(),
    ///                 channels: NonZeroU8::new(2).unwrap(),
    ///                 parameters: RtpCodecParametersParameters::from([
    ///                     ("useinbandfec", 1_u32.into()),
    ///                     ("usedtx", 1_u32.into()),
    ///                 ]),
    ///                 rtcp_feedback: vec![],
    ///             }],
    ///             header_extensions: vec![],
    ///             encodings: vec![],
    ///             rtcp: RtcpParameters::default(),
    ///         },
    ///     ))
    ///     .await?;
    ///
    /// // Pipe producer1 into router2.
    /// router1
    ///     .pipe_producer_to_router(
    ///         producer1.id(),
    ///         PipeToRouterOptions::new(router2.clone())
    ///     )
    ///     .await?;
    ///
    /// // Consume producer1 from router2.
    /// let transport2 = router2
    ///     .create_webrtc_transport(WebRtcTransportOptions::new(TransportListenIps::new(
    ///         TransportListenIp {
    ///             ip: "127.0.0.1".parse().unwrap(),
    ///             announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///         },
    ///     )))
    ///     .await?;
    /// let consumer2 = transport2
    ///     .consume(ConsumerOptions::new(
    ///         producer1.id(),
    ///         RtpCapabilities {
    ///            codecs: vec![
    ///                RtpCodecCapability::Audio {
    ///                    mime_type: MimeTypeAudio::Opus,
    ///                    preferred_payload_type: Some(100),
    ///                    clock_rate: NonZeroU32::new(48000).unwrap(),
    ///                    channels: NonZeroU8::new(2).unwrap(),
    ///                    parameters: RtpCodecParametersParameters::default(),
    ///                    rtcp_feedback: vec![],
    ///                },
    ///            ],
    ///            header_extensions: vec![],
    ///        }
    ///     ))
    ///     .await?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pipe_producer_to_router(
        &self,
        producer_id: ProducerId,
        pipe_to_router_options: PipeToRouterOptions,
    ) -> Result<PipeProducerToRouterPair, PipeProducerToRouterError> {
        debug!("pipe_producer_to_router()");

        if pipe_to_router_options.router.id() == self.id() {
            return Err(PipeProducerToRouterError::SameRouter);
        }

        let producer = match self
            .inner
            .producers
            .read()
            .get(&producer_id)
            .and_then(WeakProducer::upgrade)
        {
            Some(producer) => producer,
            None => {
                return Err(PipeProducerToRouterError::ProducerNotFound(producer_id));
            }
        };

        let pipe_transport_pair = self
            .get_or_create_pipe_transport_pair(pipe_to_router_options)
            .await?;

        let pipe_consumer = pipe_transport_pair
            .local
            .consume(ConsumerOptions::new(
                producer_id,
                RtpCapabilities::default(),
            ))
            .await?;

        let pipe_producer = pipe_transport_pair
            .remote
            .produce({
                let mut producer_options = ProducerOptions::new_pipe_transport(
                    producer_id,
                    pipe_consumer.kind(),
                    pipe_consumer.rtp_parameters().clone(),
                );
                producer_options.paused = pipe_consumer.producer_paused();
                producer_options.app_data = producer.app_data().clone();

                producer_options
            })
            .await?;

        // Pipe events from the pipe Consumer to the pipe Producer.
        pipe_consumer
            .on_close({
                let pipe_producer_weak = pipe_producer.downgrade();

                move || {
                    if let Some(pipe_producer) = pipe_producer_weak.upgrade() {
                        pipe_producer.close();
                    }
                }
            })
            .detach();
        pipe_consumer
            .on_pause({
                let executor = Arc::clone(&self.inner.executor);
                let pipe_producer_weak = pipe_producer.downgrade();

                move || {
                    if let Some(pipe_producer) = pipe_producer_weak.upgrade() {
                        executor
                            .spawn(async move {
                                let _ = pipe_producer.pause().await;
                            })
                            .detach();
                    }
                }
            })
            .detach();
        pipe_consumer
            .on_resume({
                let executor = Arc::clone(&self.inner.executor);
                let pipe_producer_weak = pipe_producer.downgrade();

                move || {
                    if let Some(pipe_producer) = pipe_producer_weak.upgrade() {
                        executor
                            .spawn(async move {
                                let _ = pipe_producer.resume().await;
                            })
                            .detach();
                    }
                }
            })
            .detach();

        // Pipe events from the pipe Producer to the pipe Consumer.
        pipe_producer
            .on_close({
                let pipe_consumer = pipe_consumer.clone();

                move || {
                    // Just to make sure consumer on local router outlives producer on the other
                    // router
                    drop(pipe_consumer);
                }
            })
            .detach();

        let pipe_producer = PipedProducer::new(pipe_producer, {
            let weak_producer = producer.downgrade();

            move |pipe_producer| {
                if let Some(producer) = weak_producer.upgrade() {
                    // In case `PipedProducer` was dropped without transforming into regular
                    // `Producer` first, we need to tie underlying pipe producer lifetime to the
                    // lifetime of original source producer in another router
                    producer
                        .on_close(move || {
                            pipe_producer.close();
                        })
                        .detach();
                }
            }
        });

        Ok(PipeProducerToRouterPair {
            pipe_consumer,
            pipe_producer,
        })
    }

    /// Pipes [`DataProducer`] with the given `data_producer_id` into another [`Router`] on same
    /// host.
    ///
    /// # Example
    /// ```rust
    /// use mediasoup::data_consumer::DataConsumerOptions;
    /// use mediasoup::data_structures::TransportListenIp;
    /// use mediasoup::data_producer::DataProducerOptions;
    /// use mediasoup::router::{PipeToRouterOptions, RouterOptions};
    /// use mediasoup::sctp_parameters::SctpStreamParameters;
    /// use mediasoup::transport::Transport;
    /// use mediasoup::webrtc_transport::{TransportListenIps, WebRtcTransportOptions};
    /// use mediasoup::worker::WorkerSettings;
    ///
    /// # async fn f(
    /// #     worker_manager: mediasoup::worker_manager::WorkerManager,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// // Have two workers.
    /// let worker1 = worker_manager.create_worker(WorkerSettings::default()).await?;
    /// let worker2 = worker_manager.create_worker(WorkerSettings::default()).await?;
    ///
    /// // Create a router in each worker.
    /// let router1 = worker1.create_router(RouterOptions::default()).await?;
    /// let router2 = worker2.create_router(RouterOptions::default()).await?;
    ///
    /// // Produce in router1.
    /// let transport1 = router1
    ///     .create_webrtc_transport({
    ///         let mut options = WebRtcTransportOptions::new(TransportListenIps::new(
    ///             TransportListenIp {
    ///                 ip: "127.0.0.1".parse().unwrap(),
    ///                 announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///             },
    ///         ));
    ///         options.enable_sctp = true;
    ///         options
    ///     })
    ///     .await?;
    /// let data_producer1 = transport1
    ///     .produce_data(DataProducerOptions::new_sctp(
    ///         SctpStreamParameters::new_unordered_with_life_time(666, 5000),
    ///     ))
    ///     .await?;
    ///
    /// // Pipe data_producer1 into router2.
    /// router1
    ///     .pipe_data_producer_to_router(
    ///         data_producer1.id(),
    ///         PipeToRouterOptions::new(router2.clone())
    ///     )
    ///     .await?;
    ///
    /// // Consume data_producer1 from router2.
    /// let transport2 = router2
    ///     .create_webrtc_transport({
    ///         let mut options = WebRtcTransportOptions::new(TransportListenIps::new(
    ///             TransportListenIp {
    ///                 ip: "127.0.0.1".parse().unwrap(),
    ///                 announced_ip: Some("9.9.9.1".parse().unwrap()),
    ///             },
    ///         ));
    ///         options.enable_sctp = true;
    ///         options
    ///     })
    ///     .await?;
    /// let data_consumer2 = transport2
    ///     .consume_data(DataConsumerOptions::new_sctp(data_producer1.id()))
    ///     .await?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pipe_data_producer_to_router(
        &self,
        data_producer_id: DataProducerId,
        pipe_to_router_options: PipeToRouterOptions,
    ) -> Result<PipeDataProducerToRouterPair, PipeDataProducerToRouterError> {
        debug!("pipe_data_producer_to_router()");

        if pipe_to_router_options.router.id() == self.id() {
            return Err(PipeDataProducerToRouterError::SameRouter);
        }

        let data_producer = match self
            .inner
            .data_producers
            .read()
            .get(&data_producer_id)
            .and_then(WeakDataProducer::upgrade)
        {
            Some(data_producer) => data_producer,
            None => {
                return Err(PipeDataProducerToRouterError::DataProducerNotFound(
                    data_producer_id,
                ));
            }
        };

        let pipe_transport_pair = self
            .get_or_create_pipe_transport_pair(pipe_to_router_options)
            .await?;

        let pipe_data_consumer = pipe_transport_pair
            .local
            .consume_data(DataConsumerOptions::new_sctp(data_producer_id))
            .await?;

        let pipe_data_producer = pipe_transport_pair
            .remote
            .produce_data({
                let mut producer_options = DataProducerOptions::new_pipe_transport(
                    data_producer_id,
                    // We've created `DataConsumer` with SCTP above, so this should never panic
                    pipe_data_consumer.sctp_stream_parameters().unwrap(),
                );
                producer_options.label = pipe_data_consumer.label().clone();
                producer_options.protocol = pipe_data_consumer.protocol().clone();
                producer_options.app_data = data_producer.app_data().clone();

                producer_options
            })
            .await?;

        // Pipe events from the pipe DataConsumer to the pipe DataProducer.
        pipe_data_consumer
            .on_close({
                let pipe_data_producer_weak = pipe_data_producer.downgrade();

                move || {
                    if let Some(pipe_data_producer) = pipe_data_producer_weak.upgrade() {
                        pipe_data_producer.close();
                    }
                }
            })
            .detach();
        // Pipe events from the pipe DataProducer to the pipe Consumer.
        pipe_data_producer
            .on_close({
                let pipe_data_consumer = pipe_data_consumer.clone();

                move || {
                    // Just to make sure consumer on local router outlives producer on the other
                    // router
                    drop(pipe_data_consumer);
                }
            })
            .detach();

        let pipe_data_producer = NonClosingDataProducer::new(pipe_data_producer, {
            let weak_data_producer = data_producer.downgrade();

            move |pipe_data_producer| {
                if let Some(data_producer) = weak_data_producer.upgrade() {
                    // In case `NonClosingDataProducer` was dropped without transforming into
                    // regular `DataProducer` first, we need to tie underlying pipe data producer
                    // lifetime to the lifetime of original source data producer in another router
                    data_producer
                        .on_close(move || {
                            pipe_data_producer.close();
                        })
                        .detach();
                }
            }
        });

        Ok(PipeDataProducerToRouterPair {
            pipe_data_consumer,
            pipe_data_producer,
        })
    }

    /// Check whether the given RTP capabilities are valid to consume the given producer.
    #[must_use]
    pub fn can_consume(
        &self,
        producer_id: &ProducerId,
        rtp_capabilities: &RtpCapabilities,
    ) -> bool {
        if let Some(producer) = self.get_producer(producer_id) {
            match ortc::can_consume(producer.consumable_rtp_parameters(), rtp_capabilities) {
                Ok(result) => result,
                Err(error) => {
                    error!("can_consume() | unexpected error: {}", error);
                    false
                }
            }
        } else {
            error!(
                "can_consume() | Producer with id \"{}\" not found",
                producer_id
            );
            false
        }
    }

    /// Callback is called when a new transport is created.
    pub fn on_new_transport<F: Fn(NewTransport<'_>) + Send + Sync + 'static>(
        &self,
        callback: F,
    ) -> HandlerId {
        self.inner.handlers.new_transport.add(Box::new(callback))
    }

    /// Callback is called when a new RTP observer is created.
    pub fn on_new_rtp_observer<F: Fn(NewRtpObserver<'_>) + Send + Sync + 'static>(
        &self,
        callback: F,
    ) -> HandlerId {
        self.inner.handlers.new_rtp_observer.add(Box::new(callback))
    }

    /// Callback is called when the worker this router belongs to is closed for whatever reason.
    /// The router itself is also closed. A `on_router_close` callbacks are triggered in all its
    /// transports all RTP observers.
    pub fn on_worker_close<F: FnOnce() + Send + 'static>(&self, callback: F) -> HandlerId {
        self.inner.handlers.worker_close.add(Box::new(callback))
    }

    /// Callback is called when the router is closed for whatever reason.
    ///
    /// NOTE: Callback will be called in place if router is already closed.
    pub fn on_close<F: FnOnce() + Send + 'static>(&self, callback: F) -> HandlerId {
        let handler_id = self.inner.handlers.close.add(Box::new(callback));
        if self.inner.closed.load(Ordering::Relaxed) {
            self.inner.handlers.close.call_simple();
        }
        handler_id
    }

    async fn get_or_create_pipe_transport_pair(
        &self,
        pipe_to_router_options: PipeToRouterOptions,
    ) -> Result<PipeTransportPair, RequestError> {
        // Here we may have to create a new PipeTransport pair to connect source and
        // destination Routers. We just want to keep a PipeTransport pair for each
        // pair of Routers. Since this operation is async, it may happen that two
        // simultaneous calls piping to the same router would end up generating two pairs of
        // `PipeTransport`. To prevent that, we have `HashMap` with mapping behind `Mutex` and
        // another `Mutex` on the pair of `PipeTransport`.

        let mut mapped_pipe_transports = self.inner.mapped_pipe_transports.lock();

        let pipe_transport_pair_mutex = mapped_pipe_transports
            .entry(pipe_to_router_options.router.id())
            .or_default()
            .clone();

        let mut pipe_transport_pair_guard = pipe_transport_pair_mutex.lock().await;

        drop(mapped_pipe_transports);

        let pipe_transport_pair = match pipe_transport_pair_guard
            .as_ref()
            .and_then(WeakPipeTransportPair::upgrade)
        {
            Some(pipe_transport_pair) => pipe_transport_pair,
            None => {
                let pipe_transport_pair = self
                    .create_pipe_transport_pair(pipe_to_router_options)
                    .await?;
                pipe_transport_pair_guard.replace(pipe_transport_pair.downgrade());

                pipe_transport_pair
            }
        };

        Ok(pipe_transport_pair)
    }

    async fn create_pipe_transport_pair(
        &self,
        pipe_to_router_options: PipeToRouterOptions,
    ) -> Result<PipeTransportPair, RequestError> {
        let PipeToRouterOptions {
            router,
            listen_ip,
            enable_sctp,
            num_sctp_streams,
            enable_rtx,
            enable_srtp,
        } = pipe_to_router_options;

        let remote_router_id = router.id();

        let transport_options = PipeTransportOptions {
            enable_sctp,
            num_sctp_streams,
            enable_rtx,
            enable_srtp,
            app_data: AppData::default(),
            ..PipeTransportOptions::new(listen_ip)
        };
        let local_pipe_transport_fut = self.create_pipe_transport(transport_options.clone());

        let remote_pipe_transport_fut = router.create_pipe_transport(transport_options);

        let (local_pipe_transport, remote_pipe_transport) =
            future::try_zip(local_pipe_transport_fut, remote_pipe_transport_fut).await?;

        let local_connect_fut = local_pipe_transport.connect({
            let tuple = remote_pipe_transport.tuple();

            PipeTransportRemoteParameters {
                ip: tuple.local_ip(),
                port: tuple.local_port(),
                srtp_parameters: remote_pipe_transport.srtp_parameters(),
            }
        });

        let remote_connect_fut = remote_pipe_transport.connect({
            let tuple = local_pipe_transport.tuple();

            PipeTransportRemoteParameters {
                ip: tuple.local_ip(),
                port: tuple.local_port(),
                srtp_parameters: local_pipe_transport.srtp_parameters(),
            }
        });

        future::try_zip(local_connect_fut, remote_connect_fut).await?;

        local_pipe_transport
            .on_close({
                let mapped_pipe_transports = Arc::clone(&self.inner.mapped_pipe_transports);

                Box::new(move || {
                    mapped_pipe_transports.lock().remove(&remote_router_id);
                })
            })
            .detach();

        remote_pipe_transport
            .on_close({
                let mapped_pipe_transports = Arc::clone(&self.inner.mapped_pipe_transports);

                Box::new(move || {
                    mapped_pipe_transports.lock().remove(&remote_router_id);
                })
            })
            .detach();

        Ok(PipeTransportPair {
            local: local_pipe_transport,
            remote: remote_pipe_transport,
        })
    }

    fn after_transport_creation(&self, transport: &impl TransportGeneric) {
        {
            let producers_weak = Arc::downgrade(&self.inner.producers);
            transport
                .on_new_producer(Box::new(move |producer| {
                    let producer_id = producer.id();
                    if let Some(producers) = producers_weak.upgrade() {
                        producers.write().insert(producer_id, producer.downgrade());
                    }
                    {
                        let producers_weak = producers_weak.clone();
                        producer
                            .on_close(move || {
                                if let Some(producers) = producers_weak.upgrade() {
                                    producers.write().remove(&producer_id);
                                }
                            })
                            .detach();
                    }
                }))
                .detach();
        }
        {
            let data_producers_weak = Arc::downgrade(&self.inner.data_producers);
            transport
                .on_new_data_producer(Box::new(move |data_producer| {
                    let data_producer_id = data_producer.id();
                    if let Some(data_producers) = data_producers_weak.upgrade() {
                        data_producers
                            .write()
                            .insert(data_producer_id, data_producer.downgrade());
                    }
                    {
                        let data_producers_weak = data_producers_weak.clone();
                        data_producer
                            .on_close(move || {
                                if let Some(data_producers) = data_producers_weak.upgrade() {
                                    data_producers.write().remove(&data_producer_id);
                                }
                            })
                            .detach();
                    }
                }))
                .detach();
        }
    }

    fn has_producer(&self, producer_id: &ProducerId) -> bool {
        self.inner.producers.read().contains_key(producer_id)
    }

    fn get_producer(&self, producer_id: &ProducerId) -> Option<Producer> {
        self.inner.producers.read().get(producer_id)?.upgrade()
    }

    fn has_data_producer(&self, data_producer_id: &DataProducerId) -> bool {
        self.inner
            .data_producers
            .read()
            .contains_key(data_producer_id)
    }

    fn get_data_producer(&self, data_producer_id: &DataProducerId) -> Option<DataProducer> {
        self.inner
            .data_producers
            .read()
            .get(data_producer_id)?
            .upgrade()
    }

    #[cfg(test)]
    fn close(&self) {
        self.inner.close();
    }
}
