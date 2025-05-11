#!/usr/bin/env node
import fs from "node:fs/promises";
import { argv, exit } from "node:process";
import {
  Worker,
  isMainThread,
  parentPort,
  workerData,
} from "node:worker_threads";
import os from "node:os";

const compute = (left, right, start, end) => {
  let result = 0;
  for (let i = start; i < end; ++i) {
    result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
  }
  return result;
};

if (!isMainThread) {
  const { left, right, start, end } = workerData;
  const res = compute(left, right, start, end);
  parentPort?.postMessage(res);
}

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

let left, right;
try {
  const file = await fs.open(argv[2], "r");
  const buffer = Buffer.alloc(size * 2 * 4); // 2 arrays, 4 bytes per int
  await file.read(buffer, 0, buffer.length, 0);
  await file.close();
  left = new Int32Array(buffer.buffer, 0, size);
  right = new Int32Array(buffer.buffer, size * 4, size);
} catch (err) {
  console.error(`Error: Could not open file ${argv[2]}`);
  exit(1);
}

const chunk = Math.ceil(size / numThreads);
const promises = [];

for (let t = 0; t < numThreads; ++t) {
  const start = t * chunk;
  const end = Math.min((t + 1) * chunk, size);
  if (start >= end) break;
  promises.push(
    new Promise((resolve, reject) => {
      const worker = new Worker(new URL(import.meta.url), {
        workerData: {
          left: left.slice(start, end),
          right: right.slice(start, end),
          start: 0,
          end: end - start,
        },
      });
      worker.on("message", resolve);
      worker.on("error", reject);
      worker.on("exit", (code) => {
        if (code !== 0)
          reject(new Error(`Worker stopped with exit code ${code}`));
      });
    })
  );
}

const results = await Promise.all(promises);
const total = results.reduce((a, b) => a + b, 0);
console.log(total);
