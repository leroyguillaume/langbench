#!/usr/bin/env node
import fs from "node:fs/promises";
import { argv, exit } from "node:process";

if (argv.length < 4) {
  console.log(`Usage: ${argv[1]} <filepath> <size>`);
  exit(1);
}

const size = Math.floor(Number(argv[3]) / 2);
if (size <= 0) {
  console.log("Error: Size must be a positive integer");
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
  console.log(`Error: Could not open file ${argv[2]}`);
  exit(1);
}

let result = 0;
for (let i = 0; i < size; ++i) {
  result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
}

console.log(result);
