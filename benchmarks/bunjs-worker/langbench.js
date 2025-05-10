#!/usr/bin/env bun
// @ts-nocheck
import os from "node:os";

const { argv, exit } = process;

if (argv.length < 4) {
  console.log(`Usage: ${argv[1]} <filepath> <size>`);
  exit(1);
}

const size = Math.floor(Number(argv[3]) / 2);
if (size <= 0) {
  console.log("Error: Size must be a positive integer");
  exit(1);
}

const numWorkers = Math.min(os.cpus().length, size);
const chunkSize = Math.ceil(size / numWorkers);

async function main() {
  try {
    // Read file efficiently with Bun's BunFile API
    const file = Bun.file(argv[2]);

    // Get file size to validate before loading
    const fileSize = await file.size;
    if (fileSize < size * 2 * 4) {
      throw new Error("File too small");
    }

    // Use arrayBuffer for efficient binary data handling
    const buffer = await file.arrayBuffer();

    // Create direct views into the buffer for zero-copy access
    const left = new Int32Array(buffer, 0, size);
    const right = new Int32Array(buffer, size * 4, size);

    // Spawn workers and distribute the workload
    const workers = [];
    const promises = [];

    for (let i = 0; i < numWorkers; i++) {
      const startIdx = i * chunkSize;
      const endIdx = Math.min(startIdx + chunkSize, size);

      if (startIdx >= size) break;

      // Create worker
      const worker = new Worker(new URL("./worker.js", import.meta.url));
      workers.push(worker);

      // Use slice to create new array buffers for transferring
      // This avoids detaching the original buffer
      const leftChunk = left.slice(startIdx, endIdx);
      const rightChunk = right.slice(startIdx, endIdx);

      // Create promise for worker result
      const promise = new Promise((resolve) => {
        worker.onmessage = (e) => resolve(e.data);
      });
      promises.push(promise);

      // Send data to worker with transferable objects for zero-copy transfer
      worker.postMessage(
        {
          leftChunk: leftChunk.buffer,
          rightChunk: rightChunk.buffer,
          startIdx,
          endIdx,
        },
        [leftChunk.buffer, rightChunk.buffer]
      );
    }

    // Collect results from all workers
    const results = await Promise.all(promises);
    const totalResult = results.reduce((acc, val) => acc + val, 0);

    // Terminate workers
    workers.forEach((worker) => worker.terminate());

    console.log(totalResult);
  } catch (err) {
    console.log(`Error: Could not process file ${argv[2]}: ${err.message}`);
    exit(1);
  }
}

main();
