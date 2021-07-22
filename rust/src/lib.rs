#![warn(rust_2018_idioms, missing_debug_implementations, missing_docs)]
//! Rust port of [mediasoup](https://github.com/versatica/mediasoup) TypeScript library!
//!
//! For general information go to readme in repository.
//!
//! # For TypeScript users
//! If you were using mediasoup in TypeScript before, most of the API should be familiar to you.
//! However, this is not one-to-one port, API was adjusted to more idiomatic Rust style leveraging
//! powerful type system and ownership system to make API more robust and more misuse-resistant.
//!
//! So you will find specific types in most places where plain strings were used, instead of
//! `close()` you will see `Drop` implementation for major entities that will close everything
//! gracefully when it goes out of scope.
//!
//! # Before you start
//! This is very low-level **library**. Which means it doesn't come with a ready to use signaling
//! mechanism or easy to customize app scaffold (see
//! [design goals](https://github.com/versatica/mediasoup/tree/v3/rust/readme.md#design-goals)).
//!
//! It is recommended to visit mediasoup website and read
//! [design overview](https://mediasoup.org/documentation/v3/mediasoup/design/) first.
//!
//! There are some requirements for building underlying C++ `mediasoup-worker`, please find them in
//! [installation instructions](https://mediasoup.org/documentation/v3/mediasoup/installation/)
//!
//! # Examples
//! There are some examples in `examples` and `examples-frontend` directories (for server- and
//! client-side respectively), you may want to look at those to get a general idea of what API looks
//! like and what needs to be done in what order (check WebSocket messages in browser DevTools for
//! better understanding of what is happening under the hood).
//!
//! # How to start
//! With that in mind, you want start with creating [`WorkerManager`](worker_manager::WorkerManager)
//! instance and then 1 or more workers. Workers a responsible for low-level job of sending media
//! and data back and forth. Each worker is backed by single-core C++ worker thread. On each worker
//! you create one or more routers that enable injection, selection and forwarding of media and data
//! through [`transport`] instances. There are a few different transports available, but most likely
//! you'll want to use [`WebRtcTransport`](webrtc_transport::WebRtcTransport) most often. With
//! transport created you can start creating [`Producer`](producer::Producer)s to send data to
//! [`Router`](router::Router) and [`Consumer`](consumer::Consumer) instances to extract data from
//! [`Router`](router::Router).
//!
//! Some of the more advanced cases involve multiple routers and even workers that can user more
//! than one core on the machine or even scale beyond single host. Check
//! [scalability page](https://mediasoup.org/documentation/v3/scalability/) of the official
//! documentation.
//!
//! Please check integration and unit tests for usage examples, they cover all major functionality
//! and are a good place to start until we have demo apps built in Rust).

pub mod data_structures;
mod macros;
mod messages;
#[doc(hidden)]
pub mod ortc;
pub mod router;
pub mod rtp_parameters;
pub mod scalability_modes;
pub mod sctp_parameters;
pub mod srtp_parameters;
pub mod supported_rtp_capabilities;
pub mod worker;
pub mod worker_manager;

pub mod audio_level_observer {
    //! An audio level observer monitors the volume of the selected audio producers.

    pub use crate::router::audio_level_observer::*;
}

pub mod active_speaker_observer {
    //! An active speaker observer monitors the speaking activity of the selected audio producers.

    pub use crate::router::active_speaker_observer::*;
}

pub mod consumer {
    //! A consumer represents an audio or video source being forwarded from a mediasoup router to an
    //! endpoint. It's created on top of a transport that defines how the media packets are carried.

    pub use crate::router::consumer::*;
}

pub mod data_consumer {
    //! A data consumer represents an endpoint capable of receiving data messages from a mediasoup
    //! [`Router`](router::Router).
    //!
    //! A data consumer can use [SCTP](https://tools.ietf.org/html/rfc4960) (AKA
    //! DataChannel) to receive those messages, or can directly receive them in the Rust application
    //! if the data consumer was created on top of a
    //! [`DirectTransport`](direct_transport::DirectTransport).

    #[cfg(doc)]
    use super::*;
    pub use crate::router::data_consumer::*;
}

pub mod producer {
    //! A producer represents an audio or video source being injected into a mediasoup router. It's
    //! created on top of a transport that defines how the media packets are carried.

    pub use crate::router::producer::*;
}

pub mod data_producer {
    //! A data producer represents an endpoint capable of injecting data messages into a mediasoup
    //! [`Router`](router::Router).
    //!
    //! A data producer can use [SCTP](https://tools.ietf.org/html/rfc4960) (AKA DataChannel) to
    //! deliver those messages, or can directly send them from the Rust application if the data
    //! producer was created on top of a [`DirectTransport`](direct_transport::DirectTransport).

    #[cfg(doc)]
    use super::*;
    pub use crate::router::data_producer::*;
}

pub mod transport {
    //! A transport connects an endpoint with a mediasoup router and enables transmission of media
    //! in both directions by means of [`Producer`](producer::Producer),
    //! [`Consumer`](consumer::Consumer), [`DataProducer`](data_producer::DataProducer) and
    //! [`DataConsumer`](data_consumer::DataConsumer) instances created on it.
    //!
    //! mediasoup implements the following transports:
    //! * [`WebRtcTransport`](webrtc_transport::WebRtcTransport)
    //! * [`PlainTransport`](plain_transport::PlainTransport)
    //! * [`PipeTransport`](pipe_transport::PipeTransport)
    //! * [`DirectTransport`](direct_transport::DirectTransport)

    #[cfg(doc)]
    use super::*;
    pub use crate::router::transport::*;
}

pub mod direct_transport {
    //! A direct transport represents a direct connection between the mediasoup Rust process and a
    //! [`Router`](router::Router) instance in a mediasoup-worker thread.
    //!
    //! A direct transport can be used to directly send and receive data messages from/to Rust by
    //! means of [`DataProducer`](data_producer::DataProducer)s and
    //! [`DataConsumer`](data_consumer::DataConsumer)s of type `Direct` created on a direct
    //! transport.
    //! Direct messages sent by a [`DataProducer`](data_producer::DataProducer) in a direct
    //! transport can be consumed by endpoints connected through a SCTP capable transport
    //! ([`WebRtcTransport`](webrtc_transport::WebRtcTransport),
    //! [`PlainTransport`](plain_transport::PlainTransport),
    //! [`PipeTransport`](pipe_transport::PipeTransport) and also by the Rust application by means
    //! of a [`DataConsumer`](data_consumer::DataConsumer) created on a [`DirectTransport`] (and
    //! vice-versa: messages sent over SCTP/DataChannel can be consumed by the Rust application by
    //! means of a [`DataConsumer`](data_consumer::DataConsumer) created on a [`DirectTransport`]).
    //!
    //! A direct transport can also be used to inject and directly consume RTP and RTCP packets in
    //! Rust by using the [`DirectProducer::send`](producer::DirectProducer::send) and
    //! [`Consumer::on_rtp`](consumer::Consumer::on_rtp) API (plus [`DirectTransport::send_rtcp`]
    //! and [`DirectTransport::on_rtcp`] API).

    #[cfg(doc)]
    use super::*;
    pub use crate::router::direct_transport::*;
}

pub mod pipe_transport {
    //! A pipe transport represents a network path through which RTP, RTCP (optionally secured with
    //! SRTP) and SCTP (DataChannel) is transmitted. Pipe transports are intended to
    //! intercommunicate two [`Router`](router::Router) instances collocated on the same host or on
    //! separate hosts.
    //!
    //! # Notes on usage
    //! When calling [`PipeTransport::consume`](transport::Transport::consume), all RTP streams of
    //! the [`Producer`](producer::Producer) are transmitted verbatim (in contrast to what happens
    //! in [`WebRtcTransport`](webrtc_transport::WebRtcTransport) and
    //! [`PlainTransport`](plain_transport::PlainTransport) in which a single and continuous RTP
    //! stream is sent to the consuming endpoint).

    #[cfg(doc)]
    use super::*;
    pub use crate::router::pipe_transport::*;
}

pub mod plain_transport {
    //! A plain transport represents a network path through which RTP, RTCP (optionally secured with
    //! SRTP) and SCTP (DataChannel) is transmitted.

    pub use crate::router::plain_transport::*;
}

pub mod rtp_observer {
    //! An RTP observer inspects the media received by a set of selected producers.
    //!
    //! mediasoup implements the following RTP observers:
    //! * [`AudioLevelObserver`](audio_level_observer::AudioLevelObserver)

    #[cfg(doc)]
    use super::*;
    pub use crate::router::rtp_observer::*;
}

pub mod webrtc_transport {
    //! A WebRTC transport represents a network path negotiated by both, a WebRTC endpoint and
    //! mediasoup, via ICE and DTLS procedures. A WebRTC transport may be used to receive media, to
    //! send media or to both receive and send. There is no limitation in mediasoup. However, due to
    //! their design, mediasoup-client and libmediasoupclient require separate WebRTC transports for
    //! sending and receiving.
    //!
    //! # Notes on usage
    //! The WebRTC transport implementation of mediasoup is
    //! [ICE Lite](https://tools.ietf.org/html/rfc5245#section-2.7), meaning that it does not
    //! initiate ICE connections but expects ICE Binding Requests from endpoints.

    pub use crate::router::webrtc_transport::*;
}
