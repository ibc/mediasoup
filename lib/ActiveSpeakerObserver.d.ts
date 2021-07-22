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
export declare class ActiveSpeakerObserver extends RtpObserver {
    /**
     * @private
     */
    constructor(params: any);
    /**
     * Observer.
     */
    get observer(): EnhancedEventEmitter;
    private _handleWorkerNotifications;
}
//# sourceMappingURL=ActiveSpeakerObserver.d.ts.map