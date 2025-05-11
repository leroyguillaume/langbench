// @ts-nocheck
// Worker script for data processing
function handleMessage(event) {
  const leftChunk = event.data.leftChunk;
  const rightChunk = event.data.rightChunk;

  // Create TypedArrays from the transferred ArrayBuffers
  // Using Int32Array directly on the buffer provides the best performance in Bun
  const left = new Int32Array(leftChunk);
  const right = new Int32Array(rightChunk);

  // Process the data
  let result = 0;
  const length = left.length;
  for (let i = 0; i < length; i++) {
    result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
  }

  // Send the result back to the main thread
  self.postMessage(result);
}

self.onmessage = handleMessage;
