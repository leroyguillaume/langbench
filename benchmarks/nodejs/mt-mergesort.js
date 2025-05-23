const fs = require('fs');
const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

function merge(arr, left, mid, right) {
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

async function mergeSortThread(arr, left, right, depth, maxDepth) {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);

        if (depth < maxDepth) {
            // Create promises for parallel execution
            const leftPromise = new Promise((resolve) => {
                const worker = new Worker(__filename, {
                    workerData: {
                        arr: arr.slice(left, mid + 1),
                        left: 0,
                        right: mid - left,
                        depth: depth + 1,
                        maxDepth: maxDepth
                    }
                });
                worker.on('message', (sortedLeft) => {
                    arr.set(sortedLeft, left);
                    resolve();
                });
            });

            const rightPromise = new Promise((resolve) => {
                const worker = new Worker(__filename, {
                    workerData: {
                        arr: arr.slice(mid + 1, right + 1),
                        left: 0,
                        right: right - (mid + 1),
                        depth: depth + 1,
                        maxDepth: maxDepth
                    }
                });
                worker.on('message', (sortedRight) => {
                    arr.set(sortedRight, mid + 1);
                    resolve();
                });
            });

            await Promise.all([leftPromise, rightPromise]);
        } else {
            // Sequential sorting for remaining depth
            await mergeSortThread(arr, left, mid, depth + 1, maxDepth);
            await mergeSortThread(arr, mid + 1, right, depth + 1, maxDepth);
        }

        merge(arr, left, mid, right);
    }
}

// Worker thread code
if (!isMainThread) {
    const { arr, left, right, depth, maxDepth } = workerData;
    const workerArr = new Int32Array(arr);
    mergeSortThread(workerArr, left, right, depth, maxDepth)
        .then(() => parentPort.postMessage(workerArr));
}

// Main thread code
if (isMainThread) {
    if (process.argv.length !== 6) {
        console.error(`Usage: ${process.argv[1]} <input_file> <num_integers> <num_cores> <output_file>`);
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
    let buffer;
    try {
        buffer = fs.readFileSync(inputFile);
    } catch (err) {
        console.error('Error opening input file');
        process.exit(1);
    }

    const arr = new Int32Array(buffer.buffer, buffer.byteOffset, numIntegers);

    // Perform parallel merge sort
    mergeSortThread(arr, 0, numIntegers - 1, 0, maxDepth)
        .then(() => {
            try {
                const outputBuffer = Buffer.from(arr.buffer, arr.byteOffset, numIntegers * 4);
                fs.writeFileSync(outputFile, outputBuffer);
            } catch (err) {
                console.error('Error writing output file');
                process.exit(1);
            }
        })
        .catch(err => {
            console.error('Error during merge sort:', err);
            process.exit(1);
        });
}
