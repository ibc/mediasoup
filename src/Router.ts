import { v4 as uuidv4 } from 'uuid';
import { AwaitQueue } from 'awaitqueue';
import { Logger } from './Logger';
import { EnhancedEventEmitter } from './EnhancedEventEmitter';
import * as ortc from './ortc';
import { InvalidStateError } from './errors';
import { Channel } from './Channel';
import { PayloadChannel } from './PayloadChannel';
import { Transport, TransportListenIp } from './Transport';
import { WebRtcTransport, WebRtcTransportOptions } from './WebRtcTransport';
import { PlainTransport, PlainTransportOptions } from './PlainTransport';
import { PipeTransport, PipeTransportOptions } from './PipeTransport';
import { DirectTransport, DirectTransportOptions } from './DirectTransport';
import { Producer } from './Producer';
import { Consumer } from './Consumer';
import { DataProducer } from './DataProducer';
import { DataConsumer } from './DataConsumer';
import { RtpObserver } from './RtpObserver';
import { ActiveSpeakerObserver, ActiveSpeakerObserverOptions } from './ActiveSpeakerObserver';
import { AudioLevelObserver, AudioLevelObserverOptions } from './AudioLevelObserver';
import { RtpCapabilities, RtpCodecCapability } from './RtpParameters';
import { NumSctpStreams } from './SctpParameters';

export type RouterOptions =
{
	/**
	 * Router media codecs.
	 */
	mediaCodecs?: RtpCodecCapability[];

	/**
	 * Custom application data.
	 */
	appData?: any;
}

export type PipeToRouterOptions =
{
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
}

export type PipeToRouterResult =
{
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
}

const logger = new Logger('Router');

export class Router extends EnhancedEventEmitter
{
	// Internal data.
	private readonly _internal:
	{
		routerId: string;
	};

	// Router data.
	private readonly _data:
	{
		rtpCapabilities: RtpCapabilities;
	}

	// Channel instance.
	private readonly _channel: Channel;

	// PayloadChannel instance.
	private readonly _payloadChannel: PayloadChannel;

	// Closed flag.
	private _closed = false;

	// Custom app data.
	private readonly _appData?: any;

	// Transports map.
	private readonly _transports: Map<string, Transport> = new Map();

	// Producers map.
	private readonly _producers: Map<string, Producer> = new Map();

	// RtpObservers map.
	private readonly _rtpObservers: Map<string, RtpObserver> = new Map();

	// DataProducers map.
	private readonly _dataProducers: Map<string, DataProducer> = new Map();

	// Router to PipeTransport map.
	private readonly _mapRouterPipeTransports: Map<Router, PipeTransport[]> = new Map();

	// AwaitQueue instance to make pipeToRouter tasks happen sequentially.
	private readonly _pipeToRouterQueue =
		new AwaitQueue({ ClosedErrorClass: InvalidStateError });

	// Observer instance.
	private readonly _observer = new EnhancedEventEmitter();

	/**
	 * @private
	 * @emits workerclose
	 * @emits @close
	 */
	constructor(
		{
			internal,
			data,
			channel,
			payloadChannel,
			appData
		}:
		{
			internal: any;
			data: any;
			channel: Channel;
			payloadChannel: PayloadChannel;
			appData?: any;
		}
	)
	{
		super();

		logger.debug('constructor()');

		this._internal = internal;
		this._data = data;
		this._channel = channel;
		this._payloadChannel = payloadChannel;
		this._appData = appData;
	}

	/**
	 * Router id.
	 */
	get id(): string
	{
		return this._internal.routerId;
	}

	/**
	 * Whether the Router is closed.
	 */
	get closed(): boolean
	{
		return this._closed;
	}

	/**
	 * RTP capabilities of the Router.
	 */
	get rtpCapabilities(): RtpCapabilities
	{
		return this._data.rtpCapabilities;
	}

	/**
	 * App custom data.
	 */
	get appData(): any
	{
		return this._appData;
	}

	/**
	 * Invalid setter.
	 */
	set appData(appData: any) // eslint-disable-line no-unused-vars
	{
		throw new Error('cannot override appData object');
	}

	/**
	 * Observer.
	 *
	 * @emits close
	 * @emits newtransport - (transport: Transport)
	 * @emits newrtpobserver - (rtpObserver: RtpObserver)
	 */
	get observer(): EnhancedEventEmitter
	{
		return this._observer;
	}

	/**
	 * Close the Router.
	 */
	close(): void
	{
		if (this._closed)
			return;

		logger.debug('close()');

		this._closed = true;

		this._channel.request('router.close', this._internal)
			.catch(() => {});

		// Close every Transport.
		for (const transport of this._transports.values())
		{
			transport.routerClosed();
		}
		this._transports.clear();

		// Clear the Producers map.
		this._producers.clear();

		// Close every RtpObserver.
		for (const rtpObserver of this._rtpObservers.values())
		{
			rtpObserver.routerClosed();
		}
		this._rtpObservers.clear();

		// Clear the DataProducers map.
		this._dataProducers.clear();

		// Clear map of Router/PipeTransports.
		this._mapRouterPipeTransports.clear();

		// Close the pipeToRouter AwaitQueue instance.
		this._pipeToRouterQueue.close();

		this.emit('@close');

		// Emit observer event.
		this._observer.safeEmit('close');
	}

	/**
	 * Worker was closed.
	 *
	 * @private
	 */
	workerClosed(): void
	{
		if (this._closed)
			return;

		logger.debug('workerClosed()');

		this._closed = true;

		// Close every Transport.
		for (const transport of this._transports.values())
		{
			transport.routerClosed();
		}
		this._transports.clear();

		// Clear the Producers map.
		this._producers.clear();

		// Close every RtpObserver.
		for (const rtpObserver of this._rtpObservers.values())
		{
			rtpObserver.routerClosed();
		}
		this._rtpObservers.clear();

		// Clear the DataProducers map.
		this._dataProducers.clear();

		// Clear map of Router/PipeTransports.
		this._mapRouterPipeTransports.clear();

		this.safeEmit('workerclose');

		// Emit observer event.
		this._observer.safeEmit('close');
	}

	/**
	 * Dump Router.
	 */
	async dump(): Promise<any>
	{
		logger.debug('dump()');

		return this._channel.request('router.dump', this._internal);
	}

	/**
	 * Create a WebRtcTransport.
	 */
	async createWebRtcTransport(
		{
			listenIps,
			port,
			enableUdp = true,
			enableTcp = false,
			preferUdp = false,
			preferTcp = false,
			initialAvailableOutgoingBitrate = 600000,
			enableSctp = false,
			numSctpStreams = { OS: 1024, MIS: 1024 },
			maxSctpMessageSize = 262144,
			sctpSendBufferSize = 262144,
			appData = {}
		}: WebRtcTransportOptions
	): Promise<WebRtcTransport>
	{
		logger.debug('createWebRtcTransport()');

		if (!Array.isArray(listenIps))
			throw new TypeError('missing listenIps');
		else if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');

		listenIps = listenIps.map((listenIp) =>
		{
			if (typeof listenIp === 'string' && listenIp)
			{
				return { ip: listenIp };
			}
			else if (typeof listenIp === 'object')
			{
				return {
					ip          : listenIp.ip,
					announcedIp : listenIp.announcedIp || undefined
				};
			}
			else
			{
				throw new TypeError('wrong listenIp');
			}
		});

		const internal = { ...this._internal, transportId: uuidv4() };
		const reqData = {
			listenIps,
			port,
			enableUdp,
			enableTcp,
			preferUdp,
			preferTcp,
			initialAvailableOutgoingBitrate,
			enableSctp,
			numSctpStreams,
			maxSctpMessageSize,
			sctpSendBufferSize,
			isDataChannel : true
		};

		const data =
			await this._channel.request('router.createWebRtcTransport', internal, reqData);

		const transport = new WebRtcTransport(
			{
				internal,
				data,
				channel                  : this._channel,
				payloadChannel           : this._payloadChannel,
				appData,
				getRouterRtpCapabilities : (): RtpCapabilities => this._data.rtpCapabilities,
				getProducerById          : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				),
				getDataProducerById : (dataProducerId: string): DataProducer | undefined => (
					this._dataProducers.get(dataProducerId)
				)
			});

		this._transports.set(transport.id, transport);
		transport.on('@close', () => this._transports.delete(transport.id));
		transport.on('@newproducer', (producer: Producer) => this._producers.set(producer.id, producer));
		transport.on('@producerclose', (producer: Producer) => this._producers.delete(producer.id));
		transport.on('@newdataproducer', (dataProducer: DataProducer) => (
			this._dataProducers.set(dataProducer.id, dataProducer)
		));
		transport.on('@dataproducerclose', (dataProducer: DataProducer) => (
			this._dataProducers.delete(dataProducer.id)
		));

		// Emit observer event.
		this._observer.safeEmit('newtransport', transport);

		return transport;
	}

	/**
	 * Create a PlainTransport.
	 */
	async createPlainTransport(
		{
			listenIp,
			port,
			rtcpMux = true,
			comedia = false,
			enableSctp = false,
			numSctpStreams = { OS: 1024, MIS: 1024 },
			maxSctpMessageSize = 262144,
			sctpSendBufferSize = 262144,
			enableSrtp = false,
			srtpCryptoSuite = 'AES_CM_128_HMAC_SHA1_80',
			appData = {}
		}: PlainTransportOptions
	): Promise<PlainTransport>
	{
		logger.debug('createPlainTransport()');

		if (!listenIp)
			throw new TypeError('missing listenIp');
		else if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');

		if (typeof listenIp === 'string' && listenIp)
		{
			listenIp = { ip: listenIp };
		}
		else if (typeof listenIp === 'object')
		{
			listenIp =
			{
				ip          : listenIp.ip,
				announcedIp : listenIp.announcedIp || undefined
			};
		}
		else
		{
			throw new TypeError('wrong listenIp');
		}

		const internal = { ...this._internal, transportId: uuidv4() };
		const reqData = {
			listenIp,
			port,
			rtcpMux,
			comedia,
			enableSctp,
			numSctpStreams,
			maxSctpMessageSize,
			sctpSendBufferSize,
			isDataChannel : false,
			enableSrtp,
			srtpCryptoSuite
		};

		const data =
			await this._channel.request('router.createPlainTransport', internal, reqData);

		const transport = new PlainTransport(
			{
				internal,
				data,
				channel                  : this._channel,
				payloadChannel           : this._payloadChannel,
				appData,
				getRouterRtpCapabilities : (): RtpCapabilities => this._data.rtpCapabilities,
				getProducerById          : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				),
				getDataProducerById : (dataProducerId: string): DataProducer | undefined => (
					this._dataProducers.get(dataProducerId)
				)
			});

		this._transports.set(transport.id, transport);
		transport.on('@close', () => this._transports.delete(transport.id));
		transport.on('@newproducer', (producer: Producer) => this._producers.set(producer.id, producer));
		transport.on('@producerclose', (producer: Producer) => this._producers.delete(producer.id));
		transport.on('@newdataproducer', (dataProducer: DataProducer) => (
			this._dataProducers.set(dataProducer.id, dataProducer)
		));
		transport.on('@dataproducerclose', (dataProducer: DataProducer) => (
			this._dataProducers.delete(dataProducer.id)
		));

		// Emit observer event.
		this._observer.safeEmit('newtransport', transport);

		return transport;
	}

	/**
	 * DEPRECATED: Use createPlainTransport().
	 */
	async createPlainRtpTransport(
		options: PlainTransportOptions
	): Promise<PlainTransport>
	{
		logger.warn(
			'createPlainRtpTransport() is DEPRECATED, use createPlainTransport()');

		return this.createPlainTransport(options);
	}

	/**
	 * Create a PipeTransport.
	 */
	async createPipeTransport(
		{
			listenIp,
			port,
			enableSctp = false,
			numSctpStreams = { OS: 1024, MIS: 1024 },
			maxSctpMessageSize = 268435456,
			sctpSendBufferSize = 268435456,
			enableRtx = false,
			enableSrtp = false,
			appData = {}
		}: PipeTransportOptions
	): Promise<PipeTransport>
	{
		logger.debug('createPipeTransport()');

		if (!listenIp)
			throw new TypeError('missing listenIp');
		else if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');

		if (typeof listenIp === 'string' && listenIp)
		{
			listenIp = { ip: listenIp };
		}
		else if (typeof listenIp === 'object')
		{
			listenIp =
			{
				ip          : listenIp.ip,
				announcedIp : listenIp.announcedIp || undefined
			};
		}
		else
		{
			throw new TypeError('wrong listenIp');
		}

		const internal = { ...this._internal, transportId: uuidv4() };
		const reqData = {
			listenIp,
			port,
			enableSctp,
			numSctpStreams,
			maxSctpMessageSize,
			sctpSendBufferSize,
			isDataChannel : false,
			enableRtx,
			enableSrtp
		};

		const data =
			await this._channel.request('router.createPipeTransport', internal, reqData);

		const transport = new PipeTransport(
			{
				internal,
				data,
				channel                  : this._channel,
				payloadChannel           : this._payloadChannel,
				appData,
				getRouterRtpCapabilities : (): RtpCapabilities => this._data.rtpCapabilities,
				getProducerById          : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				),
				getDataProducerById : (dataProducerId: string): DataProducer | undefined => (
					this._dataProducers.get(dataProducerId)
				)
			});

		this._transports.set(transport.id, transport);
		transport.on('@close', () => this._transports.delete(transport.id));
		transport.on('@newproducer', (producer: Producer) => this._producers.set(producer.id, producer));
		transport.on('@producerclose', (producer: Producer) => this._producers.delete(producer.id));
		transport.on('@newdataproducer', (dataProducer: DataProducer) => (
			this._dataProducers.set(dataProducer.id, dataProducer)
		));
		transport.on('@dataproducerclose', (dataProducer: DataProducer) => (
			this._dataProducers.delete(dataProducer.id)
		));

		// Emit observer event.
		this._observer.safeEmit('newtransport', transport);

		return transport;
	}

	/**
	 * Create a DirectTransport.
	 */
	async createDirectTransport(
		{
			maxMessageSize = 262144,
			appData = {}
		}: DirectTransportOptions =
		{
			maxMessageSize : 262144
		}
	): Promise<DirectTransport>
	{
		logger.debug('createDirectTransport()');

		const internal = { ...this._internal, transportId: uuidv4() };
		const reqData = { direct: true, maxMessageSize };

		const data =
			await this._channel.request('router.createDirectTransport', internal, reqData);

		const transport = new DirectTransport(
			{
				internal,
				data,
				channel                  : this._channel,
				payloadChannel           : this._payloadChannel,
				appData,
				getRouterRtpCapabilities : (): RtpCapabilities => this._data.rtpCapabilities,
				getProducerById          : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				),
				getDataProducerById : (dataProducerId: string): DataProducer | undefined => (
					this._dataProducers.get(dataProducerId)
				)
			});

		this._transports.set(transport.id, transport);
		transport.on('@close', () => this._transports.delete(transport.id));
		transport.on('@newproducer', (producer: Producer) => this._producers.set(producer.id, producer));
		transport.on('@producerclose', (producer: Producer) => this._producers.delete(producer.id));
		transport.on('@newdataproducer', (dataProducer: DataProducer) => (
			this._dataProducers.set(dataProducer.id, dataProducer)
		));
		transport.on('@dataproducerclose', (dataProducer: DataProducer) => (
			this._dataProducers.delete(dataProducer.id)
		));

		// Emit observer event.
		this._observer.safeEmit('newtransport', transport);

		return transport;
	}

	/**
	 * Pipes the given Producer or DataProducer into another Router in same host.
	 */
	async pipeToRouter(
		{
			producerId,
			dataProducerId,
			router,
			listenIp = '127.0.0.1',
			enableSctp = true,
			numSctpStreams = { OS: 1024, MIS: 1024 },
			enableRtx = false,
			enableSrtp = false
		}: PipeToRouterOptions
	): Promise<PipeToRouterResult>
	{
		if (!producerId && !dataProducerId)
			throw new TypeError('missing producerId or dataProducerId');
		else if (producerId && dataProducerId)
			throw new TypeError('just producerId or dataProducerId can be given');
		else if (!router)
			throw new TypeError('Router not found');
		else if (router === this)
			throw new TypeError('cannot use this Router as destination');

		let producer: Producer | undefined;
		let dataProducer: DataProducer | undefined;

		if (producerId)
		{
			producer = this._producers.get(producerId);

			if (!producer)
				throw new TypeError('Producer not found');
		}
		else if (dataProducerId)
		{
			dataProducer = this._dataProducers.get(dataProducerId);

			if (!dataProducer)
				throw new TypeError('DataProducer not found');
		}

		// Here we may have to create a new PipeTransport pair to connect source and
		// destination Routers. We just want to keep a PipeTransport pair for each
		// pair of Routers. Since this operation is async, it may happen that two
		// simultaneous calls to router1.pipeToRouter({ producerId: xxx, router: router2 })
		// would end up generating two pairs of PipeTranports. To prevent that, let's
		// use an async queue.

		let localPipeTransport: PipeTransport | undefined;
		let remotePipeTransport: PipeTransport | undefined;

		await this._pipeToRouterQueue.push(async () =>
		{
			let pipeTransportPair = this._mapRouterPipeTransports.get(router);

			if (pipeTransportPair)
			{
				localPipeTransport = pipeTransportPair[0];
				remotePipeTransport = pipeTransportPair[1];
			}
			else
			{
				try
				{
					pipeTransportPair = await Promise.all(
						[
							this.createPipeTransport(
								{ listenIp, enableSctp, numSctpStreams, enableRtx, enableSrtp }),

							router.createPipeTransport(
								{ listenIp, enableSctp, numSctpStreams, enableRtx, enableSrtp })
						]);

					localPipeTransport = pipeTransportPair[0];
					remotePipeTransport = pipeTransportPair[1];

					await Promise.all(
						[
							localPipeTransport.connect(
								{
									ip             : remotePipeTransport.tuple.localIp,
									port           : remotePipeTransport.tuple.localPort,
									srtpParameters : remotePipeTransport.srtpParameters
								}),

							remotePipeTransport.connect(
								{
									ip             : localPipeTransport.tuple.localIp,
									port           : localPipeTransport.tuple.localPort,
									srtpParameters : localPipeTransport.srtpParameters
								})
						]);

					localPipeTransport.observer.on('close', () =>
					{
						remotePipeTransport!.close();
						this._mapRouterPipeTransports.delete(router);
					});

					remotePipeTransport.observer.on('close', () =>
					{
						localPipeTransport!.close();
						this._mapRouterPipeTransports.delete(router);
					});

					this._mapRouterPipeTransports.set(
						router, [ localPipeTransport, remotePipeTransport ]);
				}
				catch (error)
				{
					logger.error(
						'pipeToRouter() | error creating PipeTransport pair:%o',
						error);

					if (localPipeTransport)
						localPipeTransport.close();

					if (remotePipeTransport)
						remotePipeTransport.close();

					throw error;
				}
			}
		});

		if (producer)
		{
			let pipeConsumer: Consumer | undefined;
			let pipeProducer: Producer | undefined;

			try
			{
				pipeConsumer = await localPipeTransport!.consume(
					{
						producerId : producerId!
					});

				pipeProducer = await remotePipeTransport!.produce(
					{
						id            : producer.id,
						kind          : pipeConsumer!.kind,
						rtpParameters : pipeConsumer!.rtpParameters,
						paused        : pipeConsumer!.producerPaused,
						appData       : producer.appData
					});

				// Ensure that the producer has not been closed in the meanwhile.
				if (producer.closed)
					throw new InvalidStateError('original Producer closed');

				// Ensure that producer.paused has not changed in the meanwhile and, if
				// so, sych the pipeProducer.
				if (pipeProducer.paused !== producer.paused)
				{
					if (producer.paused)
						await pipeProducer.pause();
					else
						await pipeProducer.resume();
				}

				// Pipe events from the pipe Consumer to the pipe Producer.
				pipeConsumer!.observer.on('close', () => pipeProducer!.close());
				pipeConsumer!.observer.on('pause', () => pipeProducer!.pause());
				pipeConsumer!.observer.on('resume', () => pipeProducer!.resume());

				// Pipe events from the pipe Producer to the pipe Consumer.
				pipeProducer.observer.on('close', () => pipeConsumer!.close());

				return { pipeConsumer, pipeProducer };
			}
			catch (error)
			{
				logger.error(
					'pipeToRouter() | error creating pipe Consumer/Producer pair:%o',
					error);

				if (pipeConsumer)
					pipeConsumer.close();

				if (pipeProducer)
					pipeProducer.close();

				throw error;
			}
		}
		else if (dataProducer)
		{
			let pipeDataConsumer: DataConsumer | undefined;
			let pipeDataProducer: DataProducer | undefined;

			try
			{
				pipeDataConsumer = await localPipeTransport!.consumeData(
					{
						dataProducerId : dataProducerId!
					});

				pipeDataProducer = await remotePipeTransport!.produceData(
					{
						id                   : dataProducer.id,
						sctpStreamParameters : pipeDataConsumer!.sctpStreamParameters,
						label                : pipeDataConsumer!.label,
						protocol             : pipeDataConsumer!.protocol,
						appData              : dataProducer.appData
					});

				// Ensure that the dataProducer has not been closed in the meanwhile.
				if (dataProducer.closed)
					throw new InvalidStateError('original DataProducer closed');

				// Pipe events from the pipe DataConsumer to the pipe DataProducer.
				pipeDataConsumer!.observer.on('close', () => pipeDataProducer!.close());

				// Pipe events from the pipe DataProducer to the pipe DataConsumer.
				pipeDataProducer.observer.on('close', () => pipeDataConsumer!.close());

				return { pipeDataConsumer, pipeDataProducer };
			}
			catch (error)
			{
				logger.error(
					'pipeToRouter() | error creating pipe DataConsumer/DataProducer pair:%o',
					error);

				if (pipeDataConsumer)
					pipeDataConsumer.close();

				if (pipeDataProducer)
					pipeDataProducer.close();

				throw error;
			}
		}
		else
		{
			throw new Error('internal error');
		}
	}

	/**
	 * Create an ActiveSpeakerObserver
	 */
	async createActiveSpeakerObserver(
		{
			interval = 300,
			appData = {}
		}: ActiveSpeakerObserverOptions = {}
	): Promise<ActiveSpeakerObserver>
	{
		logger.debug('createActiveSpeakerObserver()');

		if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');
		
		const internal = { ...this._internal, rtpObserverId: uuidv4() };
		const reqData = { interval };

		await this._channel.request('router.createActiveSpeakerObserver', internal, reqData);

		const activeSpeakerObserver = new ActiveSpeakerObserver(
			{
				internal,
				channel         : this._channel,
				payloadChannel  : this._payloadChannel,
				appData,
				getProducerById : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				)
			});
		
		this._rtpObservers.set(activeSpeakerObserver.id, activeSpeakerObserver);
		activeSpeakerObserver.on('@close', () =>
		{
			this._rtpObservers.delete(activeSpeakerObserver.id);
		});

		// Emit observer event.
		this._observer.safeEmit('newrtpobserver', activeSpeakerObserver);

		return activeSpeakerObserver;
	}

	/**
	 * Create an AudioLevelObserver.
	 */
	async createAudioLevelObserver(
		{
			maxEntries = 1,
			threshold = -80,
			interval = 1000,
			appData = {}
		}: AudioLevelObserverOptions = {}
	): Promise<AudioLevelObserver>
	{
		logger.debug('createAudioLevelObserver()');

		if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');

		const internal = { ...this._internal, rtpObserverId: uuidv4() };
		const reqData = { maxEntries, threshold, interval };

		await this._channel.request('router.createAudioLevelObserver', internal, reqData);

		const audioLevelObserver = new AudioLevelObserver(
			{
				internal,
				channel         : this._channel,
				payloadChannel  : this._payloadChannel,
				appData,
				getProducerById : (producerId: string): Producer | undefined => (
					this._producers.get(producerId)
				)
			});

		this._rtpObservers.set(audioLevelObserver.id, audioLevelObserver);
		audioLevelObserver.on('@close', () =>
		{
			this._rtpObservers.delete(audioLevelObserver.id);
		});

		// Emit observer event.
		this._observer.safeEmit('newrtpobserver', audioLevelObserver);

		return audioLevelObserver;
	}

	/**
	 * Check whether the given RTP capabilities can consume the given Producer.
	 */
	canConsume(
		{
			producerId,
			rtpCapabilities
		}:
		{
			producerId: string;
			rtpCapabilities: RtpCapabilities;
		}
	): boolean
	{
		const producer = this._producers.get(producerId);

		if (!producer)
		{
			logger.error(
				'canConsume() | Producer with id "%s" not found', producerId);

			return false;
		}

		try
		{
			return ortc.canConsume(producer.consumableRtpParameters, rtpCapabilities);
		}
		catch (error)
		{
			logger.error('canConsume() | unexpected error: %s', String(error));

			return false;
		}
	}
}
