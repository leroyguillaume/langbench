#!/usr/bin/env bun
// @ts-nocheck
import os from "node:os";
import fs from "node:fs/promises"; // Added for Node.js style file reading

const { argv, exit } = process;

if (argv.length < 5) {
  console.error(
    `Usage: ${argv[1]} <filepath> <size_of_both_arrays_elements> <threads>`
  );
  exit(1);
}

// argv[3] is the total number of elements for *both* arrays combined.
// elementsPerArray will be the number of elements in a single array.
const elementsPerArray = Math.floor(Number(argv[3]) / 2);
if (elementsPerArray <= 0) {
  console.error(
    "Error: Number of elements per array (derived from argv[3]/2) must be a positive integer"
  );
  exit(1);
}

const numThreads = Math.floor(Number(argv[4]));
if (numThreads <= 0) {
  console.error("Error: Number of threads must be a positive integer");
  exit(1);
}

const numWorkers = Math.min(numThreads, elementsPerArray);
// chunkSize is the number of elements from one array that a single worker will process.
const chunkSize = Math.ceil(elementsPerArray / numWorkers);

async function main() {
  try {
    const filePath = argv[2];
    const bytesPerElement = 4; // Assuming 4-byte numbers (e.g., Int32, Float32)
    // Total bytes needed for two arrays, each with 'elementsPerArray' elements.
    const bytesToProcess = elementsPerArray * bytesPerElement * 2;

    let sharedBuffer;

    const fileHandle = await fs.open(filePath, "r");
    try {
      const stats = await fileHandle.stat();
      if (stats.size < bytesToProcess) {
        throw new Error(
          `File too small. Needs ${bytesToProcess} bytes for processing, but file has only ${stats.size} bytes.`
        );
      }

      sharedBuffer = new SharedArrayBuffer(bytesToProcess);
      const view = new Uint8Array(sharedBuffer); // A view to read data into

      // Read directly into the SharedArrayBuffer
      const { bytesRead } = await fileHandle.read(view, 0, bytesToProcess, 0);

      if (bytesRead < bytesToProcess) {
        throw new Error(
          `Failed to read the required ${bytesToProcess} bytes. Only read ${bytesRead} bytes.`
        );
      }
    } finally {
      // Ensure the file handle is closed even if an error occurs
      await fileHandle.close();
    }

    // Spawn workers and distribute the workload
    const workers = [];
    const promises = [];

    const leftArrayByteOffsetInSAB = 0;
    const rightArrayByteOffsetInSAB = elementsPerArray * bytesPerElement;

    for (let i = 0; i < numWorkers; i++) {
      const startIdx = i * chunkSize; // element index from start of one logical array
      const endIdx = Math.min(startIdx + chunkSize, elementsPerArray);
      const chunkElementCount = endIdx - startIdx;

      if (startIdx >= elementsPerArray) break; // No more data for this array to process

      const workerPath = new URL("./worker.js", import.meta.url).href;
      const worker = new Worker(workerPath);
      workers.push(worker);

      const promise = new Promise((resolve, reject) => {
        worker.onmessage = (e) => resolve(e.data);
        worker.onerror = (err) =>
          reject(new Error(`Worker error: ${err.message}`));
        worker.onexit = (code) => {
          if (code !== 0) {
            reject(new Error(`Worker stopped with exit code ${code}`));
          }
        };
      });
      promises.push(promise);

      worker.postMessage({
        sharedBuffer,
        leftArrayByteOffsetInSAB,
        rightArrayByteOffsetInSAB,
        chunkStartElementOffset: startIdx,
        chunkElementCount,
        bytesPerElement, // Pass this to worker, it might need it
      });
    }

    const results = await Promise.all(promises);
    let totalResult = 0;
    for (const result of results) {
      totalResult += result;
    }

    for (const worker of workers) {
      worker.terminate();
    }

    console.log(totalResult);
  } catch (err) {
    console.error(`Error in main: ${err.message}`);
    // For more detailed debugging, you might want to log err.stack
    // console.error(err.stack);
    exit(1);
  }
}

main();
