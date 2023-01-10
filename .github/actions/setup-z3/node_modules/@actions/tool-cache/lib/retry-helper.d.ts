/**
 * Internal class for retries
 */
export declare class RetryHelper {
    private maxAttempts;
    private minSeconds;
    private maxSeconds;
    constructor(maxAttempts: number, minSeconds: number, maxSeconds: number);
    execute<T>(action: () => Promise<T>, isRetryable?: (e: Error) => boolean): Promise<T>;
    private getSleepAmount;
    private sleep;
}
