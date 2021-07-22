use crate::data_structures::AppData;
use crate::producer::{Producer, ProducerId};
use crate::uuid_based_wrapper_type;
use crate::worker::RequestError;
use async_trait::async_trait;
use event_listener_primitives::HandlerId;

uuid_based_wrapper_type!(
    /// [`RtpObserver`] identifier.
    RtpObserverId
);

/// Options for adding producer to `[RtpObserver`]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RtpObserverAddProducerOptions {
    /// The id of the Producer to be added.
    pub producer_id: ProducerId,
}

impl RtpObserverAddProducerOptions {
    /// * `producer_id` - The id of the [`Producer`] to be added.
    #[must_use]
    pub fn new(producer_id: ProducerId) -> Self {
        Self { producer_id }
    }
}

/// An RTP observer inspects the media received by a set of selected producers.
///
/// mediasoup implements the following RTP observers:
/// * [`AudioLevelObserver`](crate::audio_level_observer::AudioLevelObserver)
/// * [`ActiveSpeakerObserver`](crate::active_speaker_observer::ActiveSpeakerObserver)
#[async_trait(?Send)]
pub trait RtpObserver {
    /// RtpObserver id.
    #[must_use]
    fn id(&self) -> RtpObserverId;

    /// Whether the RtpObserver is paused.
    #[must_use]
    fn paused(&self) -> bool;

    /// Custom application data.
    #[must_use]
    fn app_data(&self) -> &AppData;

    /// Whether the RTP observer is closed.
    #[must_use]
    fn closed(&self) -> bool;

    /// Pauses the RTP observer. No RTP is inspected until resume() is called.
    async fn pause(&self) -> Result<(), RequestError>;

    /// Resumes the RTP observer. RTP is inspected again.
    async fn resume(&self) -> Result<(), RequestError>;

    /// Provides the RTP observer with a new producer to monitor.
    async fn add_producer(
        &self,
        rtp_observer_add_producer_options: RtpObserverAddProducerOptions,
    ) -> Result<(), RequestError>;

    /// Removes the given producer from the RTP observer.
    async fn remove_producer(&self, producer_id: ProducerId) -> Result<(), RequestError>;

    /// Callback is called when the RTP observer is paused.
    fn on_pause(&self, callback: Box<dyn Fn() + Send + Sync + 'static>) -> HandlerId;

    /// Callback is called when the RTP observer is resumed.
    fn on_resume(&self, callback: Box<dyn Fn() + Send + Sync + 'static>) -> HandlerId;

    /// Callback is called when a new producer is added into the RTP observer.
    fn on_add_producer(
        &self,
        callback: Box<dyn Fn(&Producer) + Send + Sync + 'static>,
    ) -> HandlerId;

    /// Callback is called when a producer is removed from the RTP observer.
    fn on_remove_producer(
        &self,
        callback: Box<dyn Fn(&Producer) + Send + Sync + 'static>,
    ) -> HandlerId;

    /// Callback is called when the router this RTP observer belongs to is closed for whatever reason. The RTP
    /// observer itself is also closed.
    fn on_router_close(&self, callback: Box<dyn FnOnce() + Send + 'static>) -> HandlerId;

    /// Callback is called when the RTP observer is closed for whatever reason.
    ///
    /// NOTE: Callback will be called in place if observer is already closed.
    fn on_close(&self, callback: Box<dyn FnOnce() + Send + 'static>) -> HandlerId;
}
