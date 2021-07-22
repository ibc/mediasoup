use futures_lite::future;
use mediasoup::active_speaker_observer::ActiveSpeakerObserverOptions;
use mediasoup::audio_level_observer::AudioLevelObserverOptions;
use mediasoup::consumer::{ConsumerLayers, ConsumerOptions, ConsumerTraceEventType};
use mediasoup::data_consumer::DataConsumerOptions;
use mediasoup::data_producer::DataProducerOptions;
use mediasoup::data_structures::TransportListenIp;
use mediasoup::direct_transport::DirectTransportOptions;
use mediasoup::plain_transport::PlainTransportOptions;
use mediasoup::producer::{ProducerOptions, ProducerTraceEventType};
use mediasoup::router::{PipeToRouterOptions, RouterOptions};
use mediasoup::rtp_observer::{RtpObserver, RtpObserverAddProducerOptions};
use mediasoup::rtp_parameters::{
    MediaKind, MimeTypeAudio, RtpCapabilities, RtpCodecCapability, RtpCodecParameters,
    RtpParameters,
};
use mediasoup::sctp_parameters::SctpStreamParameters;
use mediasoup::transport::{Transport, TransportGeneric, TransportTraceEventType};
use mediasoup::webrtc_transport::{TransportListenIps, WebRtcTransportOptions};
use mediasoup::worker::{WorkerLogLevel, WorkerSettings, WorkerUpdateSettings};
use mediasoup::worker_manager::WorkerManager;
use std::num::{NonZeroU32, NonZeroU8};
use std::{env, thread};

fn init() {
    {
        let mut builder = env_logger::builder();
        if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
            builder.filter_level(log::LevelFilter::Off);
        }
        let _ = builder.is_test(true).try_init();
    }
}

#[test]
fn smoke() {
    init();

    let worker_manager = WorkerManager::new();

    future::block_on(async move {
        let worker = worker_manager
            .create_worker(WorkerSettings::default())
            .await
            .unwrap();

        println!("Worker dump: {:#?}", worker.dump().await.unwrap());
        println!(
            "Update settings: {:?}",
            worker
                .update_settings({
                    let mut settings = WorkerUpdateSettings::default();

                    settings.log_level = Some(WorkerLogLevel::Debug);
                    settings.log_tags = Some(vec![]);

                    settings
                })
                .await
                .unwrap()
        );

        let router = worker
            .create_router({
                let mut router_options = RouterOptions::default();
                router_options.media_codecs = vec![RtpCodecCapability::Audio {
                    mime_type: MimeTypeAudio::Opus,
                    preferred_payload_type: None,
                    clock_rate: NonZeroU32::new(48000).unwrap(),
                    channels: NonZeroU8::new(2).unwrap(),
                    parameters: Default::default(),
                    rtcp_feedback: vec![],
                }];
                router_options
            })
            .await
            .unwrap();
        println!("Router created: {:?}", router.id());
        println!("Router dump: {:#?}", router.dump().await.unwrap());

        let webrtc_transport = router
            .create_webrtc_transport({
                let mut options =
                    WebRtcTransportOptions::new(TransportListenIps::new(TransportListenIp {
                        ip: "127.0.0.1".parse().unwrap(),
                        announced_ip: None,
                    }));

                options.enable_sctp = true;

                options
            })
            .await
            .unwrap();
        println!("WebRTC transport created: {:?}", webrtc_transport.id());
        println!(
            "WebRTC transport stats: {:#?}",
            webrtc_transport.get_stats().await.unwrap()
        );
        println!(
            "WebRTC transport dump: {:#?}",
            webrtc_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!(
            "WebRTC transport enable trace event: {:#?}",
            webrtc_transport
                .enable_trace_event(vec![TransportTraceEventType::Bwe])
                .await
                .unwrap()
        );

        let producer = webrtc_transport
            .produce(ProducerOptions::new(
                MediaKind::Audio,
                RtpParameters {
                    mid: Some("AUDIO".to_string()),
                    codecs: vec![RtpCodecParameters::Audio {
                        mime_type: MimeTypeAudio::Opus,
                        payload_type: 111,
                        clock_rate: NonZeroU32::new(48000).unwrap(),
                        channels: NonZeroU8::new(2).unwrap(),
                        parameters: Default::default(),
                        rtcp_feedback: vec![],
                    }],
                    ..RtpParameters::default()
                },
            ))
            .await
            .unwrap();

        println!("Producer created: {:?}", producer.id());
        println!(
            "WebRTC transport dump: {:#?}",
            webrtc_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());

        println!("Producer stats: {:#?}", producer.get_stats().await.unwrap());
        println!("Producer dump: {:#?}", producer.dump().await.unwrap());
        println!("Producer pause: {:#?}", producer.pause().await.unwrap());
        println!("Producer resume: {:#?}", producer.resume().await.unwrap());
        println!(
            "Producer enable trace event: {:#?}",
            producer
                .enable_trace_event(vec![ProducerTraceEventType::KeyFrame])
                .await
                .unwrap()
        );

        let consumer = webrtc_transport
            .consume(ConsumerOptions::new(
                producer.id(),
                RtpCapabilities {
                    codecs: vec![RtpCodecCapability::Audio {
                        mime_type: MimeTypeAudio::Opus,
                        preferred_payload_type: None,
                        clock_rate: NonZeroU32::new(48000).unwrap(),
                        channels: NonZeroU8::new(2).unwrap(),
                        parameters: Default::default(),
                        rtcp_feedback: vec![],
                    }],
                    header_extensions: vec![],
                },
            ))
            .await
            .unwrap();
        println!("Consumer created: {:?}", consumer.id());
        println!(
            "WebRTC transport dump: {:#?}",
            webrtc_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!("Consumer stats: {:#?}", consumer.get_stats().await.unwrap());
        println!("Producer dump: {:#?}", producer.dump().await.unwrap());
        println!("Consumer dump: {:#?}", consumer.dump().await.unwrap());
        println!("Consumer pause: {:#?}", consumer.pause().await.unwrap());
        println!("Consumer resume: {:#?}", consumer.resume().await.unwrap());
        println!(
            "Consumer set preferred layers: {:#?}",
            consumer
                .set_preferred_layers(ConsumerLayers {
                    spatial_layer: 1,
                    temporal_layer: None
                })
                .await
                .unwrap()
        );
        println!(
            "Consumer set priority: {:#?}",
            consumer.set_priority(10).await.unwrap()
        );
        println!(
            "Consumer unset priority: {:#?}",
            consumer.unset_priority().await.unwrap()
        );
        println!(
            "Consumer enable trace event: {:#?}",
            consumer
                .enable_trace_event(vec![ConsumerTraceEventType::KeyFrame])
                .await
                .unwrap()
        );

        let data_producer = webrtc_transport
            .produce_data(DataProducerOptions::new_sctp(
                SctpStreamParameters::new_ordered(1),
            ))
            .await
            .unwrap();

        println!("Data producer created: {:?}", producer.id());
        println!(
            "WebRTC transport dump: {:#?}",
            webrtc_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());

        println!(
            "Data producer stats: {:#?}",
            data_producer.get_stats().await.unwrap()
        );
        println!(
            "Data producer dump: {:#?}",
            data_producer.dump().await.unwrap()
        );

        let data_consumer = webrtc_transport
            .consume_data(DataConsumerOptions::new_sctp(data_producer.id()))
            .await
            .unwrap();

        println!("Data consumer created: {:?}", consumer.id());
        println!(
            "WebRTC transport dump: {:#?}",
            webrtc_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());

        println!(
            "Data producer stats: {:#?}",
            data_producer.get_stats().await.unwrap()
        );
        println!(
            "Data consumer stats: {:#?}",
            data_consumer.get_stats().await.unwrap()
        );
        println!(
            "Data consumer dump: {:#?}",
            data_consumer.dump().await.unwrap()
        );
        println!(
            "Data consumer get buffered amount: {:#?}",
            data_consumer.get_buffered_amount().await.unwrap()
        );
        println!(
            "Data consumer set buffered amount low threshold: {:#?}",
            data_consumer
                .set_buffered_amount_low_threshold(256)
                .await
                .unwrap()
        );

        let plain_transport = router
            .create_plain_transport({
                let mut options = PlainTransportOptions::new(TransportListenIp {
                    ip: "127.0.0.1".parse().unwrap(),
                    announced_ip: None,
                });

                options.enable_sctp = true;

                options
            })
            .await
            .unwrap();
        println!("Plain transport created: {:?}", plain_transport.id());
        println!(
            "Plain transport stats: {:#?}",
            plain_transport.get_stats().await.unwrap()
        );
        println!(
            "Plain transport dump: {:#?}",
            plain_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!(
            "Plain transport enable trace event: {:#?}",
            plain_transport
                .enable_trace_event(vec![TransportTraceEventType::Bwe])
                .await
                .unwrap()
        );

        let direct_transport = router
            .create_direct_transport(DirectTransportOptions::default())
            .await
            .unwrap();
        println!("Direct transport created: {:?}", direct_transport.id());
        println!(
            "Direct transport stats: {:#?}",
            direct_transport.get_stats().await.unwrap()
        );
        println!(
            "Direct transport dump: {:#?}",
            direct_transport.dump().await.unwrap()
        );
        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!(
            "Direct transport enable trace event: {:#?}",
            direct_transport
                .enable_trace_event(vec![TransportTraceEventType::Bwe])
                .await
                .unwrap()
        );

        let audio_level_observer = router
            .create_audio_level_observer(AudioLevelObserverOptions::default())
            .await
            .unwrap();

        println!("Audio level observer: {:#?}", audio_level_observer.id());
        println!(
            "Add producer to audio level observer: {:#?}",
            audio_level_observer
                .add_producer(RtpObserverAddProducerOptions::new(producer.id()))
                .await
                .unwrap()
        );

        let active_speaker_observer = router
            .create_active_speaker_observer(ActiveSpeakerObserverOptions::default())
            .await
            .unwrap();

        println!(
            "Active speaker observer: {:#?}",
            active_speaker_observer.id()
        );
        println!(
            "Add producer to active speaker observer: {:#?}",
            active_speaker_observer
                .add_producer(RtpObserverAddProducerOptions::new(producer.id()))
                .await
                .unwrap()
        );

        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!(
            "Remove producer from audio level observer: {:#?}",
            audio_level_observer
                .remove_producer(producer.id())
                .await
                .unwrap()
        );

        let router2 = worker
            .create_router(RouterOptions::new(vec![RtpCodecCapability::Audio {
                mime_type: MimeTypeAudio::Opus,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(48000).unwrap(),
                channels: NonZeroU8::new(2).unwrap(),
                parameters: Default::default(),
                rtcp_feedback: vec![],
            }]))
            .await
            .unwrap();
        println!("Second router created: {:?}", router.id());

        router
            .pipe_producer_to_router(producer.id(), PipeToRouterOptions::new(router2.clone()))
            .await
            .unwrap();
        println!("Piped producer to other router",);

        router
            .pipe_data_producer_to_router(
                data_producer.id(),
                PipeToRouterOptions::new(router2.clone()),
            )
            .await
            .unwrap();
        println!("Piped data producer to other router",);

        println!("Router dump: {:#?}", router.dump().await.unwrap());
        println!("Router 2 dump: {:#?}", router2.dump().await.unwrap());
    });

    // Just to give it time to finish everything
    thread::sleep(std::time::Duration::from_millis(200));
}
