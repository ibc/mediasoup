import * as process from 'process';
import * as path from 'path';
import { spawn, ChildProcess } from 'child_process';
import uuidv4 from 'uuid/v4';
import { Logger } from './Logger';
import { EnhancedEventEmitter } from './EnhancedEventEmitter';
import * as ortc from './ortc';
import { Channel } from './Channel';
import { Router, RouterOptions } from './Router';

export type WorkerLogLevel = 'debug' | 'warn' | 'error' | 'none';

export interface WorkerSettings
{
	/**
	 * Logging level for logs generated by the media worker subprocesses (check
	 * the Debugging documentation). Valid values are 'debug', 'warn', 'error' and
	 * 'none'. Default 'error'.
	 */
	logLevel?: WorkerLogLevel;

	/**
	 * Log tags for debugging. Check the list of available tags in Debugging
	 * documentation.
	 */
	logTags?: string[];

	/**
	 * Minimun RTC port for ICE, DTLS, RTP, etc. Default 10000.
	 */
	rtcMinPort?: number;

	/**
	 * Maximum RTC port for ICE, DTLS, RTP, etc. Default 59999.
	 */
	rtcMaxPort?: number;

	/**
	 * Path to the DTLS public certificate file in PEM format. If unset, a
	 * certificate is dynamically created.
	 */
	dtlsCertificateFile?: string;

	/**
	 * Path to the DTLS certificate private key file in PEM format. If unset, a
	 * certificate is dynamically created.
	 */
	dtlsPrivateKeyFile?: string;

	/**
	 * Custom application data.
	 */
	appData?: any;
}

export type WorkerUpdateableSettings = Pick<WorkerSettings, 'logLevel' | 'logTags'>;

/**
 * An object with the fields of the uv_rusage_t struct.
 *
 * - http://docs.libuv.org/en/v1.x/misc.html#c.uv_rusage_t
 * - https://linux.die.net/man/2/getrusage
 */
export interface WorkerResourceUsage
{
	/* eslint-disable camelcase */

	/**
	 * User CPU time used (in ms).
	 */
	ru_utime: number;

	/**
	 * System CPU time used (in ms).
	 */
	ru_stime: number;

	/**
	 * Maximum resident set size.
	 */
	ru_maxrss: number;

	/**
	 * Integral shared memory size.
	 */
	ru_ixrss: number;

	/**
	 * Integral unshared data size.
	 */
	ru_idrss: number;

	/**
	 * Integral unshared stack size.
	 */
	ru_isrss: number;

	/**
	 * Page reclaims (soft page faults).
	 */
	ru_minflt: number;

	/**
	 * Page faults (hard page faults).
	 */
	ru_majflt: number;

	/**
	 * Swaps.
	 */
	ru_nswap: number;

	/**
	 * Block input operations.
	 */
	ru_inblock: number;

	/**
	 * Block output operations.
	 */
	ru_oublock: number;

	/**
	 * IPC messages sent.
	 */
	ru_msgsnd: number;

	/**
	 * IPC messages received.
	 */
	ru_msgrcv: number;

	/**
	 * Signals received.
	 */
	ru_nsignals: number;

	/**
	 * Voluntary context switches.
	 */
	ru_nvcsw: number;

	/**
	 * Involuntary context switches.
	 */
	ru_nivcsw: number;

	/* eslint-enable camelcase */
}

// If env MEDIASOUP_WORKER_BIN is given, use it as worker binary.
// Otherwise if env MEDIASOUP_BUILDTYPE is 'Debug' use the Debug binary.
// Otherwise use the Release binary.
const workerBin = process.env.MEDIASOUP_WORKER_BIN
	? process.env.MEDIASOUP_WORKER_BIN
	: process.env.MEDIASOUP_BUILDTYPE === 'Debug'
		? path.join(__dirname, '..', 'worker', 'out', 'Debug', 'mediasoup-worker')
		: path.join(__dirname, '..', 'worker', 'out', 'Release', 'mediasoup-worker');

const logger = new Logger('Worker');
const workerLogger = new Logger('Worker');

export class Worker extends EnhancedEventEmitter
{
	// mediasoup-worker child process.
	private _child?: ChildProcess;

	// Worker process PID.
	private readonly _pid: number;

	// Channel instance.
	private readonly _channel: Channel;

	// Closed flag.
	private _closed = false;

	// Custom app data.
	private readonly _appData?: any;

	// Routers set.
	private readonly _routers: Set<Router> = new Set();

	// Observer instance.
	private readonly _observer = new EnhancedEventEmitter();

	/**
	 * @private
	 * @emits died - (error: Error)
	 * @emits @success
	 * @emits @failure - (error: Error)
	 */
	constructor(
		{
			logLevel,
			logTags,
			rtcMinPort,
			rtcMaxPort,
			dtlsCertificateFile,
			dtlsPrivateKeyFile,
			appData
		}: WorkerSettings)
	{
		super();

		logger.debug('constructor()');

		let spawnBin = workerBin;
		let spawnArgs: string[] = [];

		if (process.env.MEDIASOUP_USE_VALGRIND)
		{
			spawnBin = process.env.MEDIASOUP_VALGRIND_BIN || 'valgrind';

			if (process.env.MEDIASOUP_VALGRIND_OPTIONS)
				spawnArgs = spawnArgs.concat(process.env.MEDIASOUP_VALGRIND_OPTIONS.split(/\s+/));

			spawnArgs.push(workerBin);
		}

		if (typeof logLevel === 'string' && logLevel)
			spawnArgs.push(`--logLevel=${logLevel}`);

		for (const logTag of (Array.isArray(logTags) ? logTags : []))
		{
			if (typeof logTag === 'string' && logTag)
				spawnArgs.push(`--logTag=${logTag}`);
		}

		if (typeof rtcMinPort === 'number' || !Number.isNaN(parseInt(rtcMinPort)))
			spawnArgs.push(`--rtcMinPort=${rtcMinPort}`);

		if (typeof rtcMaxPort === 'number' || !Number.isNaN(parseInt(rtcMaxPort)))
			spawnArgs.push(`--rtcMaxPort=${rtcMaxPort}`);

		if (typeof dtlsCertificateFile === 'string' && dtlsCertificateFile)
			spawnArgs.push(`--dtlsCertificateFile=${dtlsCertificateFile}`);

		if (typeof dtlsPrivateKeyFile === 'string' && dtlsPrivateKeyFile)
			spawnArgs.push(`--dtlsPrivateKeyFile=${dtlsPrivateKeyFile}`);

		logger.debug(
			'spawning worker process: %s %s', spawnBin, spawnArgs.join(' '));

		this._child = spawn(
			// command
			spawnBin,
			// args
			spawnArgs,
			// options
			{
				env :
				{
					MEDIASOUP_VERSION : '__MEDIASOUP_VERSION__'
				},

				detached : false,

				// fd 0 (stdin)   : Just ignore it.
				// fd 1 (stdout)  : Pipe it for 3rd libraries that log their own stuff.
				// fd 2 (stderr)  : Same as stdout.
				// fd 3 (channel) : Producer Channel fd.
				// fd 4 (channel) : Consumer Channel fd.
				stdio : [ 'ignore', 'pipe', 'pipe', 'pipe', 'pipe' ]
			});

		this._pid = this._child.pid;

		this._channel = new Channel(
			{
				producerSocket : this._child.stdio[3],
				consumerSocket : this._child.stdio[4],
				pid            : this._pid
			});

		this._appData = appData;

		let spawnDone = false;

		// Listen for 'ready' notification.
		this._channel.once(String(this._pid), (event: string) =>
		{
			if (!spawnDone && event === 'running')
			{
				spawnDone = true;

				logger.debug('worker process running [pid:%s]', this._pid);

				this.emit('@success');
			}
		});

		this._child.on('exit', (code, signal) =>
		{
			this._child = undefined;
			this.close();

			if (!spawnDone)
			{
				spawnDone = true;

				if (code === 42)
				{
					logger.error(
						'worker process failed due to wrong settings [pid:%s]', this._pid);

					this.emit('@failure', new TypeError('wrong settings'));
				}
				else
				{
					logger.error(
						'worker process failed unexpectedly [pid:%s, code:%s, signal:%s]',
						this._pid, code, signal);

					this.emit(
						'@failure',
						new Error(`[pid:${this._pid}, code:${code}, signal:${signal}]`));
				}
			}
			else
			{
				logger.error(
					'worker process died unexpectedly [pid:%s, code:%s, signal:%s]',
					this._pid, code, signal);

				this.safeEmit(
					'died',
					new Error(`[pid:${this._pid}, code:${code}, signal:${signal}]`));
			}
		});

		this._child.on('error', (error) =>
		{
			this._child = undefined;
			this.close();

			if (!spawnDone)
			{
				spawnDone = true;

				logger.error(
					'worker process failed [pid:%s]: %s', this._pid, error.message);

				this.emit('@failure', error);
			}
			else
			{
				logger.error(
					'worker process error [pid:%s]: %s', this._pid, error.message);

				this.safeEmit('died', error);
			}
		});

		// Be ready for 3rd party worker libraries logging to stdout.
		this._child.stdout.on('data', (buffer) =>
		{
			for (const line of buffer.toString('utf8').split('\n'))
			{
				if (line)
					workerLogger.debug(`(stdout) ${line}`);
			}
		});

		// In case of a worker bug, mediasoup will log to stderr.
		this._child.stderr.on('data', (buffer) =>
		{
			for (const line of buffer.toString('utf8').split('\n'))
			{
				if (line)
					workerLogger.error(`(stderr) ${line}`);
			}
		});
	}

	/**
	 * Worker process identifier (PID).
	 */
	get pid(): number
	{
		return this._pid;
	}

	/**
	 * Whether the Worker is closed.
	 */
	get closed(): boolean
	{
		return this._closed;
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
	 * @emits newrouter - (router: Router)
	 */
	get observer(): EnhancedEventEmitter
	{
		return this._observer;
	}

	/**
	 * Close the Worker.
	 */
	close(): void
	{
		if (this._closed)
			return;

		logger.debug('close()');

		this._closed = true;

		// Kill the worker process.
		if (this._child)
		{
			// Remove event listeners but leave a fake 'error' hander to avoid
			// propagation.
			this._child.removeAllListeners('exit');
			this._child.removeAllListeners('error');
			this._child.on('error', () => {});
			this._child.kill('SIGTERM');
			this._child = undefined;
		}

		// Close the Channel instance.
		this._channel.close();

		// Close every Router.
		for (const router of this._routers)
		{
			router.workerClosed();
		}
		this._routers.clear();

		// Emit observer event.
		this._observer.safeEmit('close');
	}

	/**
	 * Dump Worker.
	 */
	async dump(): Promise<any>
	{
		logger.debug('dump()');

		return this._channel.request('worker.dump');
	}

	/**
	 * Get mediasoup-worker process resource usage.
	 */
	async getResourceUsage(): Promise<WorkerResourceUsage>
	{
		logger.debug('getResourceUsage()');

		return this._channel.request('worker.getResourceUsage');
	}

	/**
	 * Update settings.
	 */
	async updateSettings(
		{
			logLevel,
			logTags
		}: WorkerUpdateableSettings = {}
	): Promise<void>
	{
		logger.debug('updateSettings()');

		const reqData = { logLevel, logTags };

		await this._channel.request('worker.updateSettings', undefined, reqData);
	}

	/**
	 * Create a Router.
	 */
	async createRouter(
		{
			mediaCodecs,
			appData = {}
		}: RouterOptions = {}): Promise<Router>
	{
		logger.debug('createRouter()');

		if (appData && typeof appData !== 'object')
			throw new TypeError('if given, appData must be an object');

		// This may throw.
		const rtpCapabilities = ortc.generateRouterRtpCapabilities(mediaCodecs);

		const internal = { routerId: uuidv4() };

		await this._channel.request('worker.createRouter', internal);

		const data = { rtpCapabilities };
		const router = new Router(
			{
				internal,
				data,
				channel : this._channel,
				appData
			});

		this._routers.add(router);
		router.on('@close', () => this._routers.delete(router));

		// Emit observer event.
		this._observer.safeEmit('newrouter', router);

		return router;
	}
}