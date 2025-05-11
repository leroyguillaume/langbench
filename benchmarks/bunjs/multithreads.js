#!/usr/bin/env bun
// @ts-nocheck
import os from "node:os";

const { argv, exit } = process;

if (argv.length < 5) {
  console.error(`Usage: ${argv[1]} <filepath> <size> <threads>`);
  exit(1);
}

const size = Math.floor(Number(argv[3]) / 2);
if (size <= 0) {
  console.error("Error: Size must be a positive integer");
  exit(1);
}

const numThreads = Math.floor(Number(argv[4]));
if (numThreads <= 0) {
  console.error("Error: Number of threads must be a positive integer");
  exit(1);
}

const numWorkers = Math.min(numThreads, size);
const chunkSize = Math.ceil(size / numWorkers);

async function main() {
  try {
    // Read file efficiently with Bun's BunFile API
    const file = Bun.file(argv[2]);

    // Get file size to validate before loading
    const fileSize = file.size;
    if (fileSize < size * 2 * 4) {
      throw new Error("File too small");
    }

    // Use arrayBuffer for efficient binary data handling
    const buffer = await file.arrayBuffer();

    // Create a SharedArrayBuffer and copy data from the original buffer
    const sharedBuffer = new SharedArrayBuffer(buffer.byteLength);
    new Uint8Array(sharedBuffer).set(new Uint8Array(buffer));

    // Note: Original 'left' and 'right' Int32Array views on 'buffer' are no longer needed here for transfer.
    // The partitioning logic below uses 'size' (elements in one logical array) and 'chunkSize'.

    // Spawn workers and distribute the workload
    const workers = [];
    const promises = [];

    const leftArrayByteOffsetInSAB = 0;
    const rightArrayByteOffsetInSAB = size * 4; // 'size' is element count of one array

    for (let i = 0; i < numWorkers; i++) {
      const startIdx = i * chunkSize; // element index from start of logical array
      const endIdx = Math.min(startIdx + chunkSize, size);
      const chunkElementCount = endIdx - startIdx;

      if (startIdx >= size) break; // No more data to process

      const worker = new Worker(new URL("./worker.js", import.meta.url));
      workers.push(worker);

      // Create promise for worker result
      const promise = new Promise((resolve) => {
        worker.onmessage = (e) => resolve(e.data);
      });
      promises.push(promise);

      // Send data to worker. SharedArrayBuffer is shared, not transferred.
      worker.postMessage({
        sharedBuffer,
        leftArrayByteOffsetInSAB,
        rightArrayByteOffsetInSAB,
        chunkStartElementOffset: startIdx,
        chunkElementCount,
      });
    }

    // Collect results from all workers
    const results = await Promise.all(promises);
    let totalResult = 0;
    for (let i = 0; i < results.length; i++) {
      totalResult += results[i];
    }

    for (let i = 0; i < workers.length; i++) {
      workers[i].terminate();
    }

    console.log(totalResult);
  } catch (err) {
    console.error(`Error: Could not process file ${argv[2]}: ${err.message}`);
    exit(1);
  }
}

main();
