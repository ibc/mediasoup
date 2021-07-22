import { EnhancedEventEmitter } from './EnhancedEventEmitter';
import { Channel } from './Channel';
import { PayloadChannel } from './PayloadChannel';
import { TransportListenIp } from './Transport';
import { WebRtcTransport, WebRtcTransportOptions } from './WebRtcTransport';
import { PlainTransport, PlainTransportOptions } from './PlainTransport';
import { PipeTransport, PipeTransportOptions } from './PipeTransport';
import { DirectTransport, DirectTransportOptions } from './DirectTransport';
import { Producer } from './Producer';
import { Consumer } from './Consumer';
import { DataProducer } from './DataProducer';
import { DataConsumer } from './DataConsumer';
import { ActiveSpeakerObserver, ActiveSpeakerObserverOptions } from './ActiveSpeakerObserver';
import { AudioLevelObserver, AudioLevelObserverOptions } from './AudioLevelObserver';
import { RtpCapabilities, RtpCodecCapability } from './RtpParameters';
import { NumSctpStreams } from './SctpParameters';
export declare type RouterOptions = {
    /**
     * Router media codecs.
     */
    mediaCodecs?: RtpCodecCapability[];
    /**
     * Custom application data.
     */
    appData?: any;
};
export declare type PipeToRouterOptions = {
    /**
     * The id of the Producer to consume.
     */
    producerId?: string;
    /**
     * The id of the DataProducer to consume.
     */
    dataProducerId?: string;
    /**
     * Target Router instance.
     */
    router: Router;
    /**
     * IP used in the PipeTransport pair. Default '127.0.0.1'.
     */
    listenIp?: TransportListenIp | string;
    /**
     * Create a SCTP association. Default true.
     */
    enableSctp?: boolean;
    /**
     * SCTP streams number.
     */
    numSctpStreams?: NumSctpStreams;
    /**
     * Enable RTX and NACK for RTP retransmission.
     */
    enableRtx?: boolean;
    /**
     * Enable SRTP.
     */
    enableSrtp?: boolean;
};
export declare type PipeToRouterResult = {
    /**
     * The Consumer created in the current Router.
     */
    pipeConsumer?: Consumer;
    /**
     * The Producer created in the target Router.
     */
    pipeProducer?: Producer;
    /**
     * The DataConsumer created in the current Router.
     */
    pipeDataConsumer?: DataConsumer;
    /**
     * The DataProducer created in the target Router.
     */
    pipeDataProducer?: DataProducer;
};
export declare class Router extends EnhancedEventEmitter {
    private readonly _internal;
    private readonly _data;
    private readonly _channel;
    private readonly _payloadChannel;
    private _closed;
    private readonly _appData?;
    private readonly _transports;
    private readonly _producers;
    private readonly _rtpObservers;
    private readonly _dataProducers;
    private readonly _mapRouterPipeTransports;
    private readonly _pipeToRouterQueue;
    private readonly _observer;
    /**
     * @private
     * @emits workerclose
     * @emits @close
     */
    constructor({ internal, data, channel, payloadChannel, appData }: {
        internal: any;
        data: any;
        channel: Channel;
        payloadChannel: PayloadChannel;
        appData?: any;
    });
    /**
     * Router id.
     */
    get id(): string;
    /**
     * Whether the Router is closed.
     */
    get closed(): boolean;
    /**
     * RTP capabilities of the Router.
     */
    get rtpCapabilities(): RtpCapabilities;
    /**
     * App custom data.
     */
    get appData(): any;
    /**
     * Invalid setter.
     */
    set appData(appData: any);
    /**
     * Observer.
     *
     * @emits close
     * @emits newtransport - (transport: Transport)
     * @emits newrtpobserver - (rtpObserver: RtpObserver)
     */
    get observer(): EnhancedEventEmitter;
    /**
     * Close the Router.
     */
    close(): void;
    /**
     * Worker was closed.
     *
     * @private
     */
    workerClosed(): void;
    /**
     * Dump Router.
     */
    dump(): Promise<any>;
    /**
     * Create a WebRtcTransport.
     */
    createWebRtcTransport({ listenIps, port, enableUdp, enableTcp, preferUdp, preferTcp, initialAvailableOutgoingBitrate, enableSctp, numSctpStreams, maxSctpMessageSize, sctpSendBufferSize, appData }: WebRtcTransportOptions): Promise<WebRtcTransport>;
    /**
     * Create a PlainTransport.
     */
    createPlainTransport({ listenIp, port, rtcpMux, comedia, enableSctp, numSctpStreams, maxSctpMessageSize, sctpSendBufferSize, enableSrtp, srtpCryptoSuite, appData }: PlainTransportOptions): Promise<PlainTransport>;
    /**
     * DEPRECATED: Use createPlainTransport().
     */
    createPlainRtpTransport(options: PlainTransportOptions): Promise<PlainTransport>;
    /**
     * Create a PipeTransport.
     */
    createPipeTransport({ listenIp, port, enableSctp, numSctpStreams, maxSctpMessageSize, sctpSendBufferSize, enableRtx, enableSrtp, appData }: PipeTransportOptions): Promise<PipeTransport>;
    /**
     * Create a DirectTransport.
     */
    createDirectTransport({ maxMessageSize, appData }?: DirectTransportOptions): Promise<DirectTransport>;
    /**
     * Pipes the given Producer or DataProducer into another Router in same host.
     */
    pipeToRouter({ producerId, dataProducerId, router, listenIp, enableSctp, numSctpStreams, enableRtx, enableSrtp }: PipeToRouterOptions): Promise<PipeToRouterResult>;
    /**
     * Create an ActiveSpeakerObserver
     */
    createActiveSpeakerObserver({ interval, appData }?: ActiveSpeakerObserverOptions): Promise<ActiveSpeakerObserver>;
    /**
     * Create an AudioLevelObserver.
     */
    createAudioLevelObserver({ maxEntries, threshold, interval, appData }?: AudioLevelObserverOptions): Promise<AudioLevelObserver>;
    /**
     * Check whether the given RTP capabilities can consume the given Producer.
     */
    canConsume({ producerId, rtpCapabilities }: {
        producerId: string;
        rtpCapabilities: RtpCapabilities;
    }): boolean;
}
//# sourceMappingURL=Router.d.ts.map