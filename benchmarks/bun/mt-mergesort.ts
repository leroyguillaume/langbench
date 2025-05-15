import { readFileSync, writeFileSync } from 'fs';
import { Worker, isMainThread, parentPort, workerData } from 'worker_threads';

function merge(arr: Int32Array, left: number, mid: number, right: number): void {
    const n1 = mid - left + 1;
    const n2 = right - mid;

    // Create temporary arrays
    const L = new Int32Array(n1);
    const R = new Int32Array(n2);

    // Copy data to temporary arrays
    for (let i = 0; i < n1; i++) {
        L[i] = arr[left + i];
    }
    for (let j = 0; j < n2; j++) {
        R[j] = arr[mid + 1 + j];
    }

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
            const leftPromise = new Promise<void>((resolve) => {
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
            });

            const rightPromise = new Promise<void>((resolve) => {
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
            });

            await Promise.all([leftPromise, rightPromise]);
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
        .then(() => parentPort?.postMessage('done'));
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

    // Calculate max depth for thread creation
    let maxDepth = 0;
    let temp = numCores;
    while (temp > 1) {
        maxDepth++;
        temp = Math.floor(temp / 2);
    }

    // Read input file
    const buffer = readFileSync(inputFile);
    const sharedBuffer = new SharedArrayBuffer(buffer.byteLength);
    const arr = new Int32Array(sharedBuffer);
    arr.set(new Int32Array(buffer.buffer, buffer.byteOffset, numIntegers));

    // Perform parallel merge sort
    mergeSortParallel(arr, 0, numIntegers - 1, 0, maxDepth)
        .then(() => {
            // Write output file
            writeFileSync(outputFile, Buffer.from(arr.buffer));
        });
}
