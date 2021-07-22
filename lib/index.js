"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const Logger_1 = require("./Logger");
const EnhancedEventEmitter_1 = require("./EnhancedEventEmitter");
const Worker_1 = require("./Worker");
const utils = require("./utils");
const supportedRtpCapabilities_1 = require("./supportedRtpCapabilities");
const types = require("./types");
exports.types = types;
/**
 * Expose mediasoup version.
 */
exports.version = '3.7.19';
/**
 * Expose parseScalabilityMode() function.
 */
var scalabilityModes_1 = require("./scalabilityModes");
exports.parseScalabilityMode = scalabilityModes_1.parse;
const logger = new Logger_1.Logger();
const observer = new EnhancedEventEmitter_1.EnhancedEventEmitter();
exports.observer = observer;
/**
 * Create a Worker.
 */
async function createWorker({ logLevel = 'error', logTags, rtcMinPort = 10000, rtcMaxPort = 59999, sendNackDelayMs = 0, dtlsCertificateFile, dtlsPrivateKeyFile, appData = {} } = {}) {
    logger.debug('createWorker()');
    if (appData && typeof appData !== 'object')
        throw new TypeError('if given, appData must be an object');
    const worker = new Worker_1.Worker({
        logLevel,
        logTags,
        rtcMinPort,
        rtcMaxPort,
        sendNackDelayMs,
        dtlsCertificateFile,
        dtlsPrivateKeyFile,
        appData
    });
    return new Promise((resolve, reject) => {
        worker.on('@success', () => {
            // Emit observer event.
            observer.safeEmit('newworker', worker);
            resolve(worker);
        });
        worker.on('@failure', reject);
    });
}
exports.createWorker = createWorker;
/**
 * Get a cloned copy of the mediasoup supported RTP capabilities.
 */
function getSupportedRtpCapabilities() {
    return utils.clone(supportedRtpCapabilities_1.supportedRtpCapabilities);
}
exports.getSupportedRtpCapabilities = getSupportedRtpCapabilities;
