import { readFileSync, writeFileSync } from 'fs';
import { Worker, isMainThread, parentPort, workerData } from 'worker_threads';

function merge(arr: Int32Array, left: number, mid: number, right: number): void {
    const n1 = mid - left + 1;
    const n2 = right - mid;

    // Create temporary arrays
    const L = new Int32Array(n1);
    const R = new Int32Array(n2);

    // Copy data to temporary arrays using memcpy-like approach
    L.set(arr.slice(left, left + n1));
    R.set(arr.slice(mid + 1, mid + 1 + n2));

    // Merge the temporary arrays back
    let i = 0, j = 0, k = left;
    while (i < n1 && j < n2) {
        if (L[i] <= R[j]) {
            arr[k] = L[i];
            i++;
        } else {
            arr[k] = R[j];
            j++;
        }
        k++;
    }

    // Copy remaining elements of L[]
    while (i < n1) {
        arr[k] = L[i];
        i++;
        k++;
    }

    // Copy remaining elements of R[]
    while (j < n2) {
        arr[k] = R[j];
        j++;
        k++;
    }
}

async function mergeSortParallel(arr: Int32Array, left: number, right: number, depth: number, maxDepth: number): Promise<void> {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);

        if (depth < maxDepth) {
            // Create promises for parallel execution
            const [leftResult, rightResult] = await Promise.all([
                new Promise<void>((resolve, reject) => {
                    const worker = new Worker(new URL(import.meta.url), {
                        workerData: {
                            arr: arr.buffer,
                            left: left,
                            right: mid,
                            depth: depth + 1,
                            maxDepth: maxDepth
                        }
                    });
                    worker.on('message', () => resolve());
                    worker.on('error', reject);
                }),
                new Promise<void>((resolve, reject) => {
                    const worker = new Worker(new URL(import.meta.url), {
                        workerData: {
                            arr: arr.buffer,
                            left: mid + 1,
                            right: right,
                            depth: depth + 1,
                            maxDepth: maxDepth
                        }
                    });
                    worker.on('message', () => resolve());
                    worker.on('error', reject);
                })
            ]);
        } else {
            // Sequential sorting for remaining depth
            await mergeSortParallel(arr, left, mid, depth + 1, maxDepth);
            await mergeSortParallel(arr, mid + 1, right, depth + 1, maxDepth);
        }

        merge(arr, left, mid, right);
    }
}

// Worker thread code
if (!isMainThread) {
    const { arr, left, right, depth, maxDepth } = workerData;
    const sharedArr = new Int32Array(arr);
    mergeSortParallel(sharedArr, left, right, depth, maxDepth)
        .then(() => parentPort?.postMessage('done'))
        .catch(error => {
            console.error('Worker error:', error);
            process.exit(1);
        });
}

// Main thread code
if (isMainThread) {
    if (process.argv.length !== 6) {
        console.error('Usage: bun mt-mergesort.ts <input_file> <num_integers> <num_cores> <output_file>');
        process.exit(1);
    }

    const inputFile = process.argv[2];
    const numIntegers = parseInt(process.argv[3]);
    const numCores = parseInt(process.argv[4]);
    const outputFile = process.argv[5];

    try {
        // Calculate max depth for thread creation
        let maxDepth = 0;
        let temp = numCores;
        while (temp > 1) {
            maxDepth++;
            temp = Math.floor(temp / 2);
        }

        // Read input file
        const buffer = readFileSync(inputFile);
        if (buffer.length < numIntegers * 4) {
            console.error('Error: Input file is too small');
            process.exit(1);
        }

        // Create shared array and copy data
        const sharedBuffer = new SharedArrayBuffer(numIntegers * 4);
        const arr = new Int32Array(sharedBuffer);
        const inputArr = new Int32Array(buffer.buffer, buffer.byteOffset, numIntegers);
        arr.set(inputArr);

        // Perform parallel merge sort
        await mergeSortParallel(arr, 0, numIntegers - 1, 0, maxDepth);

        // Write output file directly, similar to C's fwrite
        writeFileSync(outputFile, Buffer.from(arr.buffer, arr.byteOffset, arr.byteLength));
    } catch (error) {
        console.error('Error:', error.message);
        process.exit(1);
    }
}
