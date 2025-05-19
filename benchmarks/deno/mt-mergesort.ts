#!/usr/bin/env -S deno run --allow-read --allow-write --allow-net

// @ts-ignore
declare const Deno: {
    args: string[];
    readFile(path: string): Promise<Uint8Array>;
    writeFile(path: string, data: Uint8Array): Promise<void>;
    exit(code: number): never;
};

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

function mergeSortSequential(arr: Int32Array, left: number, right: number): void {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);
        mergeSortSequential(arr, left, mid);
        mergeSortSequential(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

async function parallelMergeSort(arr: Int32Array, numThreads: number): Promise<void> {
    const len = arr.length;
    if (len <= 1) {
        return;
    }

    // For small arrays, use sequential sort
    if (len < 10000) {
        mergeSortSequential(arr, 0, len - 1);
        return;
    }

    // Calculate chunk size for each thread
    const chunkSize = Math.ceil(len / numThreads);
    const chunks: { start: number; end: number }[] = [];

    // Divide array into chunks
    for (let i = 0; i < len; i += chunkSize) {
        chunks.push({
            start: i,
            end: Math.min(i + chunkSize - 1, len - 1)
        });
    }

    // Sort chunks in parallel using Promise.all
    await Promise.all(chunks.map(chunk => {
        return new Promise<void>(resolve => {
            // Use setTimeout to allow other chunks to be processed
            setTimeout(() => {
                mergeSortSequential(arr, chunk.start, chunk.end);
                resolve();
            }, 0);
        });
    }));

    // Merge chunks
    let chunkSize2 = chunkSize;
    while (chunkSize2 < len) {
        for (let i = 0; i < len; i += chunkSize2 * 2) {
            const mid = i + chunkSize2 - 1;
            const right = Math.min(i + chunkSize2 * 2 - 1, len - 1);
            if (mid < right) {
                merge(arr, i, mid, right);
            }
        }
        chunkSize2 *= 2;
    }
}

async function main() {
    if (Deno.args.length !== 4) {
        console.error("Usage: deno run --allow-read --allow-write --allow-net mt-mergesort.ts <input_file> <num_integers> <num_cores> <output_file>");
        Deno.exit(1);
    }

    const [inputFile, numIntegersStr, numCoresStr, outputFile] = Deno.args;
    const numIntegers = parseInt(numIntegersStr, 10);
    const numCores = parseInt(numCoresStr, 10);

    // Read input file
    const inputData = await Deno.readFile(inputFile);
    const arr = new Int32Array(inputData.buffer);

    // Perform parallel merge sort
    await parallelMergeSort(arr, numCores);

    // Write output file
    await Deno.writeFile(outputFile, new Uint8Array(arr.buffer));
}

// @ts-ignore
if (import.meta.main) {
    main();
}
