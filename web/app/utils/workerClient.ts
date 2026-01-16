class AlthreadWorkerClient {
    private worker: Worker | null = null;
    private nextId = 0;
    private pendingRequests: Map<number, { resolve: (val: any) => void, reject: (err: any) => void }> = new Map();

    constructor() {
        if (typeof window !== 'undefined') {
            this.worker = new Worker(new URL('../workers/althread.worker.ts', import.meta.url), { type: 'module' });
            this.worker!.onmessage = (event: MessageEvent) => {
                const { id, result, error, rawError } = event.data;
                const pending = this.pendingRequests.get(id);
                if (pending) {
                    this.pendingRequests.delete(id);
                    if (error) {
                        // Reconstruct error object if possible
                        const err = rawError || new Error(error);
                        if (rawError && typeof rawError === 'object') {
                            Object.assign(err, rawError);
                        }
                        pending.reject(err);
                    } else {
                        pending.resolve(result);
                    }
                }
            };
        }
    }

    private sendRequest(type: string, payload: any): Promise<any> {
        if (!this.worker) return Promise.reject(new Error('Worker not initialized'));

        const id = this.nextId++;
        return new Promise((resolve, reject) => {
            this.pendingRequests.set(id, { resolve, reject });
            this.worker!.postMessage({ type, payload, id });
        });
    }

    async run(source: string, filePath: string, virtualFS: any) {
        return this.sendRequest('run', { source, filePath, virtualFS });
    }

    async check(source: string, filePath: string, virtualFS: any, maxStates?: number) {
        return this.sendRequest('check', { source, filePath, virtualFS, maxStates });
    }

    async compile(source: string, filePath: string, virtualFS: any) {
        return this.sendRequest('compile', { source, filePath, virtualFS });
    }
}

export const workerClient = new AlthreadWorkerClient();
