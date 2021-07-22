use crate::active_speaker_observer::ActiveSpeakerObserverOptions;
use crate::router::RouterOptions;
use crate::rtp_observer::RtpObserver;
use crate::worker::{Worker, WorkerSettings};
use crate::worker_manager::WorkerManager;
use futures_lite::future;
use std::env;

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
fn router_close_event() {
    future::block_on(async move {
        let worker = init().await;

        let router = worker
            .create_router(RouterOptions::default())
            .await
            .expect("Failed to create router");

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .expect("Failed to create ActiveSpeakerObserver");

        let (mut close_tx, close_rx) = async_oneshot::oneshot::<()>();
        let _handler = active_speaker_observer.on_close(Box::new(move || {
            let _ = close_tx.send(());
        }));

        let (mut router_close_tx, router_close_rx) = async_oneshot::oneshot::<()>();
        let _handler = active_speaker_observer.on_router_close(Box::new(move || {
            let _ = router_close_tx.send(());
        }));

        router.close();

        router_close_rx
            .await
            .expect("Failed to receive router_close event");
        close_rx.await.expect("Failed to receive close event");

        assert_eq!(active_speaker_observer.closed(), true);
    });
}
