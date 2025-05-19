#!/usr/bin/env -S deno run --allow-read --allow-write --allow-net

// @ts-ignore
declare const Deno: {
    args: string[];
    readFile(path: string): Promise<Uint8Array>;
    writeFile(path: string, data: Uint8Array): Promise<void>;
    exit(code: number): never;
};

// @ts-ignore
declare global {
    interface ImportMeta {
        main: boolean;
        url: string;
    }
}

// @ts-ignore
declare const self: {
    name: string;
    onmessage: ((e: MessageEvent) => void) | null;
    postMessage(data: any): void;
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

async function mergeSortParallel(arr: Int32Array, left: number, right: number, depth: number, maxDepth: number): Promise<void> {
    // Use sequential sort for small arrays or when max depth is reached
    if (right - left < 10000 || depth >= maxDepth) {
        mergeSortSequential(arr, left, right);
        return;
    }

    const mid = Math.floor(left + (right - left) / 2);

    // Create promises for parallel execution
    const leftPromise = new Promise<void>((resolve) => {
        const worker = new Worker(new URL(import.meta.url).href, {
            type: "module"
        });
        // Send only the necessary slice of the array
        const leftSlice = arr.slice(left, mid + 1);
        worker.postMessage({
            arr: leftSlice,
            left: 0,
            right: leftSlice.length - 1,
            depth: depth + 1,
            maxDepth: maxDepth
        });
        worker.onmessage = (e) => {
            const sortedLeft = new Int32Array(e.data);
            for (let i = 0; i < sortedLeft.length; i++) {
                arr[left + i] = sortedLeft[i];
            }
            resolve();
        };
    });

    const rightPromise = new Promise<void>((resolve) => {
        const worker = new Worker(new URL(import.meta.url).href, {
            type: "module"
        });
        // Send only the necessary slice of the array
        const rightSlice = arr.slice(mid + 1, right + 1);
        worker.postMessage({
            arr: rightSlice,
            left: 0,
            right: rightSlice.length - 1,
            depth: depth + 1,
            maxDepth: maxDepth
        });
        worker.onmessage = (e) => {
            const sortedRight = new Int32Array(e.data);
            for (let i = 0; i < sortedRight.length; i++) {
                arr[mid + 1 + i] = sortedRight[i];
            }
            resolve();
        };
    });

    await Promise.all([leftPromise, rightPromise]);
    merge(arr, left, mid, right);
}

// Worker thread code
if (self.name !== "main") {
    self.onmessage = async (e) => {
        const { arr, left, right, depth, maxDepth } = e.data;
        const workerArr = new Int32Array(arr);
        await mergeSortParallel(workerArr, left, right, depth, maxDepth);
        self.postMessage(workerArr);
    };
}

// Main thread code
async function main() {
    if (Deno.args.length !== 4) {
        console.error("Usage: deno run --allow-read --allow-write --allow-net mt-mergesort.ts <input_file> <num_integers> <num_cores> <output_file>");
        Deno.exit(1);
    }

    const [inputFile, numIntegersStr, numCoresStr, outputFile] = Deno.args;
    const numIntegers = parseInt(numIntegersStr, 10);
    const numCores = parseInt(numCoresStr, 10);

    // Calculate max depth for thread creation
    // Limit max depth to prevent too many concurrent workers
    const maxDepth = Math.min(Math.floor(Math.log2(numCores)), 3);

    // Read input file
    const inputData = await Deno.readFile(inputFile);
    const arr = new Int32Array(inputData.buffer, inputData.byteOffset, numIntegers);

    // Perform parallel merge sort
    await mergeSortParallel(arr, 0, numIntegers - 1, 0, maxDepth);

    // Write output file
    const outputBuffer = new Uint8Array(arr.buffer, arr.byteOffset, numIntegers * 4);
    await Deno.writeFile(outputFile, outputBuffer);
}

if (import.meta.main) {
    main();
}
