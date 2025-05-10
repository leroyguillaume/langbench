// @ts-nocheck
// Worker script for data processing

self.onmessage = (event) => {
  const { leftChunk, rightChunk } = event.data;

  // Create TypedArrays from the transferred ArrayBuffers
  // Using Int32Array directly on the buffer provides the best performance in Bun
  const left = new Int32Array(leftChunk);
  const right = new Int32Array(rightChunk);

  // Process the data
  let result = 0;
  const length = left.length;

  // Unroll the loop for better performance with larger chunks
  const UNROLL_FACTOR = 4;
  const mainLoopLimit = length - (length % UNROLL_FACTOR);

  // Main loop with unrolling
  for (let i = 0; i < mainLoopLimit; i += UNROLL_FACTOR) {
    result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
    result += Math.sqrt(
      Math.abs(Math.cos(left[i + 1]) * Math.sin(right[i + 1]))
    );
    result += Math.sqrt(
      Math.abs(Math.cos(left[i + 2]) * Math.sin(right[i + 2]))
    );
    result += Math.sqrt(
      Math.abs(Math.cos(left[i + 3]) * Math.sin(right[i + 3]))
    );
  }

  // Remaining elements
  for (let i = mainLoopLimit; i < length; i++) {
    result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
  }

  // Send the result back to the main thread
  self.postMessage(result);
};
