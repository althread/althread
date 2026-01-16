import init, { initialize, run, check, compile } from '../../pkg/althread_web';

let wasmPromise: Promise<any> | null = null;

async function ensureInitialized() {
    if (!wasmPromise) {
        wasmPromise = init().then(() => {
            initialize();
        });
    }
    await wasmPromise;
}

self.onmessage = async (event) => {
    const { type, payload, id } = event.data;

    try {
        await ensureInitialized();

        let result;
        switch (type) {
            case 'run':
                result = run(payload.source, payload.filePath, payload.virtualFS);
                break;
            case 'check':
                result = check(payload.source, payload.filePath, payload.virtualFS, payload.maxStates);
                break;
            case 'compile':
                result = compile(payload.source, payload.filePath, payload.virtualFS);
                break;
            default:
                throw new Error(`Unknown task type: ${type}`);
        }
        self.postMessage({ id, result });
    } catch (error: any) {
        self.postMessage({ 
            id, 
            error: error.message || error.toString(),
            rawError: typeof error === 'object' ? {
                message: error.message,
                pos: error.pos,
                error_type: error.error_type,
                stack: error.stack
            } : error
        });
    }
};
