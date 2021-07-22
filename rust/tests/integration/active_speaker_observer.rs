use async_io::Timer;
use futures_lite::future;
use mediasoup::active_speaker_observer::ActiveSpeakerObserverOptions;
use mediasoup::router::RouterOptions;
use mediasoup::rtp_observer::RtpObserver;
use mediasoup::rtp_parameters::{MimeTypeAudio, RtpCodecCapability, RtpCodecParametersParameters};
use mediasoup::worker::{Worker, WorkerSettings};
use mediasoup::worker_manager::WorkerManager;
use std::env;
use std::num::{NonZeroU32, NonZeroU8};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn media_codecs() -> Vec<RtpCodecCapability> {
    vec![RtpCodecCapability::Audio {
        mime_type: MimeTypeAudio::Opus,
        preferred_payload_type: None,
        clock_rate: NonZeroU32::new(48000).unwrap(),
        channels: NonZeroU8::new(2).unwrap(),
        parameters: RtpCodecParametersParameters::from([
            ("useinbandfec", 1_u32.into()),
            ("foo", "bar".into()),
        ]),
        rtcp_feedback: vec![],
    }]
}

async fn init() -> Worker {
    {
        let mut builder = env_logger::builder();
        if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
            builder.filter_level(log::LevelFilter::Off);
        }
        let _ = builder.is_test(true).try_init();
    }

    let worker_manager = WorkerManager::new();

    worker_manager
        .create_worker(WorkerSettings::default())
        .await
        .expect("Failed to create worker")
}

#[test]
fn create() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .expect("Failed to create router");

        let new_observer_count = Arc::new(AtomicUsize::new(0));

        router
            .on_new_rtp_observer({
                let new_observer_count = Arc::clone(&new_observer_count);

                move |_new_rtp_observer| {
                    new_observer_count.fetch_add(1, Ordering::SeqCst);
                }
            })
            .detach();

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        assert_eq!(new_observer_count.load(Ordering::SeqCst), 1);
        assert_eq!(active_speaker_observer.closed(), false);
        assert_eq!(active_speaker_observer.paused(), false);

        let dump = router.dump().await.expect("Failed to get router dump");

        assert_eq!(
            dump.rtp_observer_ids.into_iter().collect::<Vec<_>>(),
            vec![active_speaker_observer.id()]
        );
    });
}

#[test]
fn weak() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .expect("Failed to create router");

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        let weak_active_speaker_observer = active_speaker_observer.downgrade();

        assert!(weak_active_speaker_observer.upgrade().is_some());

        drop(active_speaker_observer);

        assert!(weak_active_speaker_observer.upgrade().is_none());
    });
}

#[test]
fn pause_resume() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .expect("Failed to create router");

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        active_speaker_observer
            .pause()
            .await
            .expect("Failed to pause");
        assert_eq!(active_speaker_observer.paused(), true);

        active_speaker_observer
            .resume()
            .await
            .expect("Failed to resume");
        assert_eq!(active_speaker_observer.paused(), false);
    });
}

#[test]
fn close_event() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .expect("Failed to create router");

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        let (mut tx, rx) = async_oneshot::oneshot::<()>();
        let _handler = active_speaker_observer.on_close(Box::new(move || {
            let _ = tx.send(());
        }));
        drop(active_speaker_observer);

        rx.await.expect("Failed to receive close event");
    });
}

#[test]
fn drop_test() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .expect("Failed to create router");

        let _active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        let active_speaker_observer_2 = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        let dump = router.dump().await.expect("Failed to get router dump");

        assert_eq!(dump.rtp_observer_ids.len(), 2);

        drop(active_speaker_observer_2);

        // Drop is async, give it a bit of time to finish
        Timer::after(Duration::from_millis(200)).await;

        let dump = router.dump().await.expect("Failed to get router dump");

        assert_eq!(dump.rtp_observer_ids.len(), 1);
    });
}
