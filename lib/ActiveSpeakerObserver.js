"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const Logger_1 = require("./Logger");
const RtpObserver_1 = require("./RtpObserver");
const logger = new Logger_1.Logger('ActiveSpeakerObserver');
class ActiveSpeakerObserver extends RtpObserver_1.RtpObserver {
    /**
     * @private
     */
    constructor(params) {
        super(params);
        this._handleWorkerNotifications();
    }
    /**
     * Observer.
     */
    get observer() {
        return this._observer;
    }
    _handleWorkerNotifications() {
        this._channel.on(this._internal.rtpObserverId, (event, data) => {
            switch (event) {
                case 'dominantspeaker':
                    {
                        const dominantSpeaker = {
                            producer: this._getProducerById(data.producerId)
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
exports.ActiveSpeakerObserver = ActiveSpeakerObserver;
