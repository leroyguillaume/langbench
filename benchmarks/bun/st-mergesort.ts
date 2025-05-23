import { readFileSync, writeFileSync } from 'fs';

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

function mergeSort(arr: Int32Array, left: number, right: number): void {
    if (left < right) {
        const mid = Math.floor(left + (right - left) / 2);
        mergeSort(arr, left, mid);
        mergeSort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

// Main function
if (process.argv.length !== 5) {
    console.error('Usage: bun st-mergesort.ts <input_file> <num_integers> <output_file>');
    process.exit(1);
}

const inputFile = process.argv[2];
const numIntegers = parseInt(process.argv[3]);
const outputFile = process.argv[4];

try {
    // Read input file
    const buffer = readFileSync(inputFile);
    if (buffer.length < numIntegers * 4) {
        console.error('Error: Input file is too small');
        process.exit(1);
    }

    // Create array directly from buffer, similar to C's fread
    const arr = new Int32Array(buffer.buffer, buffer.byteOffset, numIntegers);

    // Perform merge sort
    mergeSort(arr, 0, numIntegers - 1);

    // Write output file directly, similar to C's fwrite
    writeFileSync(outputFile, Buffer.from(arr.buffer, arr.byteOffset, arr.byteLength));
} catch (error) {
    console.error('Error:', error.message);
    process.exit(1);
}
