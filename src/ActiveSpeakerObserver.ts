import { Logger } from './Logger';
import { EnhancedEventEmitter } from './EnhancedEventEmitter';
import { RtpObserver } from './RtpObserver';
import { Producer } from './Producer';

export interface ActiveSpeakerObserverOptions {
	interval?: number;
	/**
	 * Custom application data.
	 */
	appData?: any;
}

export interface ActiveSpeakerObserverActivity {
	/**
	 * The producer instance.
	 */
	producer: Producer;
}

const logger = new Logger('ActiveSpeakerObserver');

export class ActiveSpeakerObserver extends RtpObserver
{
	/**
	 * @private
	 */
	constructor(params: any)
	{
		super(params);

		this._handleWorkerNotifications();
	}

	/**
	 * Observer.
	 */
	get observer(): EnhancedEventEmitter
	{
		return this._observer;
	}

	private _handleWorkerNotifications(): void
	{
		this._channel.on(this._internal.rtpObserverId, (event: string, data?: any) =>
		{
			switch (event)
			{
				case 'dominantspeaker':
				{
					const dominantSpeaker = {
						producer : this._getProducerById(data.producerId)
					};

					this.safeEmit('dominantspeaker', dominantSpeaker);
					this._observer.safeEmit('dominantspeaker', dominantSpeaker);

					break;
				}

				default:
				{
					logger.error('ignoring unknown event "%s"', event);
				}
			}
		});
	}
}
