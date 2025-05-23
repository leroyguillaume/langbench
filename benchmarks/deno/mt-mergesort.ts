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

interface ThreadArgs {
    arr: Int32Array;
    left: number;
    right: number;
    depth: number;
    maxDepth: number;
}

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

async function mergeSortThread(args: ThreadArgs): Promise<void> {
    const { arr, left, right, depth, maxDepth } = args;

    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);

        if (depth < maxDepth) {
            // Create workers for left and right halves
            const leftPromise = new Promise<void>((resolve) => {
                const worker = new Worker(new URL(import.meta.url).href, { type: "module" });
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
                    worker.terminate();
                    resolve();
                };
            });

            const rightPromise = new Promise<void>((resolve) => {
                const worker = new Worker(new URL(import.meta.url).href, { type: "module" });
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
                    worker.terminate();
                    resolve();
                };
            });

            await Promise.all([leftPromise, rightPromise]);
        } else {
            // Sequential sorting for remaining depth
            await mergeSortThread({
                arr,
                left,
                right: mid,
                depth: depth + 1,
                maxDepth
            });
            await mergeSortThread({
                arr,
                left: mid + 1,
                right,
                depth: depth + 1,
                maxDepth
            });
        }

        merge(arr, left, mid, right);
    }
}

// Worker thread code
if (self.name !== "main") {
    self.onmessage = async (e) => {
        const args: ThreadArgs = e.data;
        await mergeSortThread(args);
        self.postMessage(args.arr);
    };
}

// Main thread code
async function main() {
    if (Deno.args.length !== 4) {
        console.error(`Usage: deno run --allow-read --allow-write mt-mergesort.ts <input_file> <num_integers> <num_cores> <output_file>`);
        Deno.exit(1);
    }

    const [inputFile, numIntegersStr, numCoresStr, outputFile] = Deno.args;
    const numIntegers = parseInt(numIntegersStr, 10);
    const numCores = parseInt(numCoresStr, 10);

    if (isNaN(numIntegers) || numIntegers <= 0 || isNaN(numCores) || numCores <= 0) {
        console.error("Invalid number of integers or cores");
        Deno.exit(1);
    }

    // Calculate max depth for thread creation
    let maxDepth = 0;
    let temp = numCores;
    while (temp > 1) {
        maxDepth++;
        temp = Math.floor(temp / 2);
    }

    try {
        // Read input file
        const inputData = await Deno.readFile(inputFile);
        if (inputData.length < numIntegers * 4) {
            console.error("Error reading input file: insufficient data");
            Deno.exit(1);
        }

        const arr = new Int32Array(inputData.buffer, inputData.byteOffset, numIntegers);

        // Perform parallel merge sort
        await mergeSortThread({
            arr,
            left: 0,
            right: numIntegers - 1,
            depth: 0,
            maxDepth
        });

        // Write output file
        const outputBuffer = new Uint8Array(arr.buffer, arr.byteOffset, numIntegers * 4);
        await Deno.writeFile(outputFile, outputBuffer);
    } catch (error) {
        console.error(`Error: ${error.message}`);
        Deno.exit(1);
    }
}

if (import.meta.main) {
    main();
}
