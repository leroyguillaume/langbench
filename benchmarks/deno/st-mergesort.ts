#!/usr/bin/env -S deno run --allow-read --allow-write

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
    }
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

function mergeSort(arr: Int32Array, left: number, right: number): void {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);
        mergeSort(arr, left, mid);
        mergeSort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

async function main() {
    if (Deno.args.length !== 3) {
        console.error("Usage: deno run --allow-read --allow-write st-mergesort.ts <input_file> <num_integers> <output_file>");
        Deno.exit(1);
    }

    const [inputFile, numIntegersStr, outputFile] = Deno.args;
    const numIntegers = parseInt(numIntegersStr, 10);

    if (isNaN(numIntegers) || numIntegers <= 0) {
        console.error("Invalid number of integers");
        Deno.exit(1);
    }

    try {
        // Read input file
        const inputData = await Deno.readFile(inputFile);
        if (inputData.length < numIntegers * 4) {
            console.error("Error reading input file: insufficient data");
            Deno.exit(1);
        }

        const arr = new Int32Array(inputData.buffer, inputData.byteOffset, numIntegers);

        // Perform merge sort
        mergeSort(arr, 0, numIntegers - 1);

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
