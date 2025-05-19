const fs = require('fs');
const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

function merge(arr, left, mid, right) {
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

async function mergeSortParallel(arr, left, right, depth, maxDepth) {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);

        if (depth < maxDepth) {
            // Create promises for parallel execution
            const leftPromise = new Promise((resolve) => {
                const worker = new Worker(__filename, {
                    workerData: {
                        arr: Array.from(arr.slice(left, mid + 1)), // Convert to regular array for transfer
                        left: 0,
                        right: mid - left,
                        depth: depth + 1,
                        maxDepth: maxDepth
                    }
                });
                worker.on('message', (sortedLeft) => {
                    // Copy sorted data back to original array
                    for (let i = 0; i < sortedLeft.length; i++) {
                        arr[left + i] = sortedLeft[i];
                    }
                    resolve();
                });
            });

            const rightPromise = new Promise((resolve) => {
                const worker = new Worker(__filename, {
                    workerData: {
                        arr: Array.from(arr.slice(mid + 1, right + 1)), // Convert to regular array for transfer
                        left: 0,
                        right: right - (mid + 1),
                        depth: depth + 1,
                        maxDepth: maxDepth
                    }
                });
                worker.on('message', (sortedRight) => {
                    // Copy sorted data back to original array
                    for (let i = 0; i < sortedRight.length; i++) {
                        arr[mid + 1 + i] = sortedRight[i];
                    }
                    resolve();
                });
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
    const workerArr = new Int32Array(arr); // Convert back to Int32Array
    mergeSortParallel(workerArr, left, right, depth, maxDepth)
        .then(() => parentPort.postMessage(Array.from(workerArr))); // Convert back to regular array for transfer
}

// Main thread code
if (isMainThread) {
    if (process.argv.length !== 6) {
        console.error('Usage: node mt-mergesort.js <input_file> <num_integers> <num_cores> <output_file>');
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
    const buffer = fs.readFileSync(inputFile);
    const arr = new Int32Array(buffer.buffer, buffer.byteOffset, numIntegers);

    // Perform parallel merge sort
    mergeSortParallel(arr, 0, numIntegers - 1, 0, maxDepth)
        .then(() => {
            // Write output file
            const outputBuffer = Buffer.from(arr.buffer, arr.byteOffset, numIntegers * 4);
            fs.writeFileSync(outputFile, outputBuffer);
        });
}
