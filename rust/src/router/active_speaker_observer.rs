#[cfg(test)]
mod tests;

use crate::data_structures::AppData;
use crate::messages::{
    RtpObserverAddProducerRequest, RtpObserverAddRemoveProducerRequestData,
    RtpObserverCloseRequest, RtpObserverInternal, RtpObserverPauseRequest,
    RtpObserverRemoveProducerRequest, RtpObserverResumeRequest,
};
use crate::producer::{Producer, ProducerId};
use crate::router::Router;
use crate::rtp_observer::{RtpObserver, RtpObserverAddProducerOptions, RtpObserverId};
use crate::worker::{Channel, RequestError, SubscriptionHandler};
use async_executor::Executor;
use async_trait::async_trait;
use event_listener_primitives::{Bag, BagOnce, HandlerId};
use log::{debug, error};
use parking_lot::Mutex;
use serde::Deserialize;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

/// [`ActiveSpeakerObserver`] options
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ActiveSpeakerObserverOptions {
    /// Interval in ms for checking audio volumes.
    /// Default 300.
    pub interval: u16,
    /// Custom application data.
    pub app_data: AppData,
}

impl Default for ActiveSpeakerObserverOptions {
    fn default() -> Self {
        Self {
            interval: 300,
            app_data: AppData::default(),
        }
    }
}

/// Represents volume of one audio producer.
#[derive(Debug, Clone)]
pub struct ActiveSpeakerObserverDominantSpeaker {
    /// The audio producer instance.
    pub producer: Producer,
}

#[derive(Default)]
struct Handlers {
    dominant_speaker: Bag<Box<dyn Fn(&ActiveSpeakerObserverDominantSpeaker) + Send + Sync>>,
    pause: Bag<Box<dyn Fn() + Send + Sync>>,
    resume: Bag<Box<dyn Fn() + Send + Sync>>,
    add_producer: Bag<Box<dyn Fn(&Producer) + Send + Sync>>,
    remove_producer: Bag<Box<dyn Fn(&Producer) + Send + Sync>>,
    router_close: BagOnce<Box<dyn FnOnce() + Send>>,
    close: BagOnce<Box<dyn FnOnce() + Send>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DominantSpeakerNotification {
    producer_id: ProducerId,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase", content = "data")]
enum Notification {
    DominantSpeaker(DominantSpeakerNotification),
}

struct Inner {
    id: RtpObserverId,
    executor: Arc<Executor<'static>>,
    channel: Channel,
    handlers: Arc<Handlers>,
    paused: AtomicBool,
    app_data: AppData,
    // Make sure router is not dropped until this audio level observer is not dropped
    router: Router,
    closed: AtomicBool,
    // Drop subscription to audio level observer-specific notifications when observer itself is
    // dropped
    _subscription_handler: Option<SubscriptionHandler>,
    _on_router_close_handler: Mutex<HandlerId>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        debug!("drop()");

        self.close(true);
    }
}

impl Inner {
    fn close(&self, close_request: bool) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            debug!("close()");

            self.handlers.close.call_simple();

            if close_request {
                let channel = self.channel.clone();
                let request = RtpObserverCloseRequest {
                    internal: RtpObserverInternal {
                        router_id: self.router.id(),
                        rtp_observer_id: self.id,
                    },
                };
                let router = self.router.clone();
                self.executor
                    .spawn(async move {
                        if let Err(error) = channel.request(request).await {
                            error!("active speaker observer closing failed on drop: {}", error);
                        }

                        drop(router);
                    })
                    .detach();
            }
        }
    }
}

/// An active speaker observer monitors the volume of the selected audio producers.
///
/// It just handles audio producers (if [`AudioLevelObserver::add_producer()`] is called with a
/// video producer it will fail).
///
/// Audio levels are read from an RTP header extension. No decoding of audio data is done. See
/// [RFC6464](https://tools.ietf.org/html/rfc6464) for more information.
#[derive(Clone)]
#[must_use = "Audio level observer will be closed on drop, make sure to keep it around for as long as needed"]
pub struct ActiveSpeakerObserver {
    inner: Arc<Inner>,
}

impl fmt::Debug for ActiveSpeakerObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveSpeakerObserver")
            .field("id", &self.inner.id)
            .field("paused", &self.inner.paused)
            .field("router", &self.inner.router)
            .field("closed", &self.inner.closed)
            .finish()
    }
}

#[async_trait(?Send)]
impl RtpObserver for ActiveSpeakerObserver {
    fn id(&self) -> RtpObserverId {
        self.inner.id
    }

    fn paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    fn app_data(&self) -> &AppData {
        &self.inner.app_data
    }

    fn closed(&self) -> bool {
        self.inner.closed.load(Ordering::SeqCst)
    }

    async fn pause(&self) -> Result<(), RequestError> {
        debug!("pause()");

        self.inner
            .channel
            .request(RtpObserverPauseRequest {
                internal: self.get_internal(),
            })
            .await?;

        let was_paused = self.inner.paused.swap(true, Ordering::SeqCst);

        if !was_paused {
            self.inner.handlers.pause.call_simple();
        }

        Ok(())
    }

    async fn resume(&self) -> Result<(), RequestError> {
        debug!("resume()");

        self.inner
            .channel
            .request(RtpObserverResumeRequest {
                internal: self.get_internal(),
            })
            .await?;

        let was_paused = self.inner.paused.swap(false, Ordering::SeqCst);

        if !was_paused {
            self.inner.handlers.resume.call_simple();
        }

        Ok(())
    }

    async fn add_producer(
        &self,
        RtpObserverAddProducerOptions { producer_id }: RtpObserverAddProducerOptions,
    ) -> Result<(), RequestError> {
        let producer = match self.inner.router.get_producer(&producer_id) {
            Some(producer) => producer,
            None => {
                return Ok(());
            }
        };
        self.inner
            .channel
            .request(RtpObserverAddProducerRequest {
                internal: self.get_internal(),
                data: RtpObserverAddRemoveProducerRequestData { producer_id },
            })
            .await?;

        self.inner.handlers.add_producer.call(|callback| {
            callback(&producer);
        });

        Ok(())
    }

    async fn remove_producer(&self, producer_id: ProducerId) -> Result<(), RequestError> {
        let producer = match self.inner.router.get_producer(&producer_id) {
            Some(producer) => producer,
            None => {
                return Ok(());
            }
        };
        self.inner
            .channel
            .request(RtpObserverRemoveProducerRequest {
                internal: self.get_internal(),
                data: RtpObserverAddRemoveProducerRequestData { producer_id },
            })
            .await?;

        self.inner.handlers.remove_producer.call(|callback| {
            callback(&producer);
        });

        Ok(())
    }

    fn on_pause(&self, callback: Box<dyn Fn() + Send + Sync + 'static>) -> HandlerId {
        self.inner.handlers.pause.add(Box::new(callback))
    }

    fn on_resume(&self, callback: Box<dyn Fn() + Send + Sync + 'static>) -> HandlerId {
        self.inner.handlers.resume.add(Box::new(callback))
    }

    fn on_add_producer(
        &self,
        callback: Box<dyn Fn(&Producer) + Send + Sync + 'static>,
    ) -> HandlerId {
        self.inner.handlers.add_producer.add(Box::new(callback))
    }

    fn on_remove_producer(
        &self,
        callback: Box<dyn Fn(&Producer) + Send + Sync + 'static>,
    ) -> HandlerId {
        self.inner.handlers.remove_producer.add(Box::new(callback))
    }

    fn on_router_close(&self, callback: Box<dyn FnOnce() + Send + 'static>) -> HandlerId {
        self.inner.handlers.router_close.add(Box::new(callback))
    }

    fn on_close(&self, callback: Box<dyn FnOnce() + Send + 'static>) -> HandlerId {
        let handler_id = self.inner.handlers.close.add(Box::new(callback));
        if self.inner.closed.load(Ordering::Relaxed) {
            self.inner.handlers.close.call_simple();
        }
        handler_id
    }
}

impl ActiveSpeakerObserver {
    pub(super) fn new(
        id: RtpObserverId,
        executor: Arc<Executor<'static>>,
        channel: Channel,
        app_data: AppData,
        router: Router,
    ) -> Self {
        debug!("new()");

        let handlers = Arc::<Handlers>::default();
        let paused = AtomicBool::new(false);

        let subscription_handler = {
            let router = router.clone();
            let handlers = Arc::clone(&handlers);

            channel.subscribe_to_notifications(id.into(), move |notification| {
                match serde_json::from_value::<Notification>(notification) {
                    Ok(notification) => match notification {
                        Notification::DominantSpeaker(dominant_speaker) => {
                            let DominantSpeakerNotification { producer_id } = dominant_speaker;
                            match router.get_producer(&producer_id) {
                                Some(producer) => {
                                    let dominant_speaker =
                                        ActiveSpeakerObserverDominantSpeaker { producer };

                                    handlers.dominant_speaker.call(|callback| {
                                        callback(&dominant_speaker);
                                    });
                                }
                                None => {
                                    error!(
                                        "Producer for dominant speaker event not found: {}",
                                        producer_id
                                    );
                                }
                            };
                        }
                    },
                    Err(error) => {
                        error!("Failed to parse notification: {}", error);
                    }
                }
            })
        };

        let inner_weak = Arc::<Mutex<Option<Weak<Inner>>>>::default();
        let on_router_close_handler = router.on_close({
            let inner_weak = Arc::clone(&inner_weak);

            move || {
                if let Some(inner) = inner_weak.lock().as_ref().and_then(Weak::upgrade) {
                    inner.handlers.router_close.call_simple();
                    inner.close(false);
                }
            }
        });
        let inner = Arc::new(Inner {
            id,
            executor,
            channel,
            handlers,
            paused,
            app_data,
            router,
            closed: AtomicBool::new(false),
            _subscription_handler: subscription_handler,
            _on_router_close_handler: Mutex::new(on_router_close_handler),
        });

        inner_weak.lock().replace(Arc::downgrade(&inner));

        Self { inner }
    }

    /// Callback is called at most every interval (see [`ActiveSpeakerObserverOptions`]).
    pub fn on_dominant_speaker<
        F: Fn(&ActiveSpeakerObserverDominantSpeaker) + Send + Sync + 'static,
    >(
        &self,
        callback: F,
    ) -> HandlerId {
        self.inner.handlers.dominant_speaker.add(Box::new(callback))
    }

    /// Downgrade `ActiveSpeakerObserver` to [`WeakActiveSpeakerObserver`] instance.
    #[must_use]
    pub fn downgrade(&self) -> WeakActiveSpeakerObserver {
        WeakActiveSpeakerObserver {
            inner: Arc::downgrade(&self.inner),
        }
    }

    fn get_internal(&self) -> RtpObserverInternal {
        RtpObserverInternal {
            router_id: self.inner.router.id(),
            rtp_observer_id: self.inner.id,
        }
    }
}

/// [`WeakActiveSpeakerObserver`] doesn't own audio level observer instance on mediasoup-worker and
/// will not prevent one from being destroyed once last instance of regular [`ActiveSpeakerObserver`]
/// is dropped.
///
/// [`WeakActiveSpeakerObserver`] vs [`ActiveSpeakerObserver`] is similar to [`Weak`] vs [`Arc`].
#[derive(Clone)]
pub struct WeakActiveSpeakerObserver {
    inner: Weak<Inner>,
}

impl fmt::Debug for WeakActiveSpeakerObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakActiveSpeakerObserver").finish()
    }
}

impl WeakActiveSpeakerObserver {
    /// Attempts to upgrade `WeakActiveSpeakerObserver` to [`ActiveSpeakerObserver`] if last instance of one wasn't
    /// dropped yet.
    #[must_use]
    pub fn upgrade(&self) -> Option<ActiveSpeakerObserver> {
        let inner = self.inner.upgrade()?;

        Some(ActiveSpeakerObserver { inner })
    }
}
