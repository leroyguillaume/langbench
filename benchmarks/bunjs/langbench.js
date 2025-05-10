#!/usr/bin/env bun
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

let left, right;
try {
  // @ts-ignore
  const file = Bun.file(argv[2]);
  // @ts-ignore
  const buffer = await file.arrayBuffer();
  if (buffer.byteLength < size * 2 * 4) {
    throw new Error("File too small");
  }
  left = new Int32Array(buffer, 0, size);
  right = new Int32Array(buffer, size * 4, size);
} catch (err) {
  console.log(`Error: Could not open file ${argv[2]}`);
  exit(1);
}

let result = 0;
for (let i = 0; i < size; ++i) {
  // @ts-ignore
  result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
}

console.log(result);
