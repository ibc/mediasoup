const os = require('os');
const process = require('process');
const { toBeType } = require('jest-tobetype');
const mediasoup = require('../');
const { createWorker, observer } = mediasoup;
const { InvalidStateError } = require('../lib/errors');

expect.extend({ toBeType });

let worker;

beforeEach(() => worker && !worker.closed && worker.close());
afterEach(() => worker && !worker.closed && worker.close());

test('createWorker() succeeds', async () =>
{
	const onObserverNewWorker = jest.fn();

	observer.once('newworker', onObserverNewWorker);

	worker = await createWorker();

	expect(onObserverNewWorker).toHaveBeenCalledTimes(1);
	expect(onObserverNewWorker).toHaveBeenCalledWith(worker);
	expect(worker).toBeType('object');
	expect(worker.pid).toBeType('number');
	expect(worker.closed).toBe(false);

	worker.close();
	expect(worker.closed).toBe(true);

	// eslint-disable-next-line require-atomic-updates
	worker = await createWorker(
		{
			logLevel            : 'debug',
			logTags             : [ 'info' ],
			rtcMinPort          : 0,
			rtcMaxPort          : 9999,
			sendNackDelayMs     : 0,
			dtlsCertificateFile : 'test/data/dtls-cert.pem',
			dtlsPrivateKeyFile  : 'test/data/dtls-key.pem',
			appData             : { bar: 456 }
		});
	expect(worker).toBeType('object');
	expect(worker.pid).toBeType('number');
	expect(worker.closed).toBe(false);
	expect(worker.appData).toEqual({ bar: 456 });

	worker.close();
	expect(worker.closed).toBe(true);
}, 2000);

test('createWorker() with wrong settings rejects with TypeError', async () =>
{
	await expect(createWorker({ logLevel: 'chicken' }))
		.rejects
		.toThrow(TypeError);

	await expect(createWorker({ rtcMinPort: 1000, rtcMaxPort: 999 }))
		.rejects
		.toThrow(TypeError);

	// Port is from 0 to 65535.
	await expect(createWorker({ rtcMinPort: 1000, rtcMaxPort: 65536 }))
		.rejects
		.toThrow(TypeError);

	await expect(createWorker({ dtlsCertificateFile: '/notfound/cert.pem' }))
		.rejects
		.toThrow(TypeError);

	await expect(createWorker({ dtlsPrivateKeyFile: '/notfound/priv.pem' }))
		.rejects
		.toThrow(TypeError);

	await expect(createWorker({ appData: 'NOT-AN-OBJECT' }))
		.rejects
		.toThrow(TypeError);
}, 2000);

test('worker.updateSettings() succeeds', async () =>
{
	worker = await createWorker();

	await expect(worker.updateSettings({ logLevel: 'debug', logTags: [ 'ice' ] }))
		.resolves
		.toBeUndefined();

	worker.close();
}, 2000);

test('worker.updateSettings() with wrong settings rejects with TypeError', async () =>
{
	worker = await createWorker();

	await expect(worker.updateSettings({ logLevel: 'chicken' }))
		.rejects
		.toThrow(TypeError);

	worker.close();
}, 2000);

test('worker.updateSettings() rejects with InvalidStateError if closed', async () =>
{
	worker = await createWorker();
	worker.close();

	await expect(worker.updateSettings({ logLevel: 'error' }))
		.rejects
		.toThrow(InvalidStateError);

	worker.close();
}, 2000);

test('worker.dump() succeeds', async () =>
{
	worker = await createWorker();

	await expect(worker.dump())
		.resolves
		.toEqual({ pid: worker.pid, routerIds: [] });

	worker.close();
}, 2000);

test('worker.dump() rejects with InvalidStateError if closed', async () =>
{
	worker = await createWorker();
	worker.close();

	await expect(worker.dump())
		.rejects
		.toThrow(InvalidStateError);

	worker.close();
}, 2000);

test('worker.getResourceUsage() succeeds', async () =>
{
	worker = await createWorker();

	await expect(worker.getResourceUsage())
		.resolves
		.toMatchObject({});

	worker.close();
}, 2000);

test('worker.close() succeeds', async () =>
{
	worker = await createWorker({ logLevel: 'warn' });

	const onObserverClose = jest.fn();

	worker.observer.once('close', onObserverClose);
	worker.close();

	expect(onObserverClose).toHaveBeenCalledTimes(1);
	expect(worker.closed).toBe(true);
}, 2000);

test('Worker emits "died" if worker process died unexpectedly', async () =>
{
	let onObserverClose;

	worker = await createWorker({ logLevel: 'warn' });
	onObserverClose = jest.fn();

	worker.observer.once('close', onObserverClose);

	await new Promise((resolve) =>
	{
		worker.on('died', resolve);

		process.kill(worker.pid, 'SIGINT');
	});

	expect(onObserverClose).toHaveBeenCalledTimes(1);
	expect(worker.closed).toBe(true);

	// eslint-disable-next-line require-atomic-updates
	worker = await createWorker({ logLevel: 'warn' });
	onObserverClose = jest.fn();

	worker.observer.once('close', onObserverClose);

	await new Promise((resolve) =>
	{
		worker.on('died', resolve);

		process.kill(worker.pid, 'SIGTERM');
	});

	expect(onObserverClose).toHaveBeenCalledTimes(1);
	expect(worker.closed).toBe(true);

	// eslint-disable-next-line require-atomic-updates
	worker = await createWorker({ logLevel: 'warn' });
	onObserverClose = jest.fn();

	worker.observer.once('close', onObserverClose);

	await new Promise((resolve) =>
	{
		worker.on('died', resolve);

		process.kill(worker.pid, 'SIGKILL');
	});

	expect(onObserverClose).toHaveBeenCalledTimes(1);
	expect(worker.closed).toBe(true);
}, 5000);

test('worker process ignores PIPE, HUP, ALRM, USR1 and USR2 signals', async () =>
{
	// Windows doesn't have some signals such as SIGPIPE, SIGALRM, SIGUSR1, SIGUSR2
	// so we just skip this test in Windows.
	if (os.platform() === 'win32')
		return;

	worker = await createWorker({ logLevel: 'warn' });

	await new Promise((resolve, reject) =>
	{
		worker.on('died', reject);

		process.kill(worker.pid, 'SIGPIPE');
		process.kill(worker.pid, 'SIGHUP');
		process.kill(worker.pid, 'SIGALRM');
		process.kill(worker.pid, 'SIGUSR1');
		process.kill(worker.pid, 'SIGUSR2');

		setTimeout(() =>
		{
			expect(worker.closed).toBe(false);

			worker.close();
			resolve();
		}, 2000);
	});
}, 3000);
