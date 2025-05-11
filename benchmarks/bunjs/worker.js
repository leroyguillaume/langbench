// @ts-nocheck
// Worker script for data processing
function handleMessage(event) {
  const {
    sharedBuffer,
    leftArrayByteOffsetInSAB,
    rightArrayByteOffsetInSAB,
    chunkStartElementOffset,
    chunkElementCount,
  } = event.data;

  // Create TypedArray views directly onto the SharedArrayBuffer
  // for the assigned chunk
  const left = new Int32Array(
    sharedBuffer,
    leftArrayByteOffsetInSAB + chunkStartElementOffset * 4, // Calculate byte offset for the chunk
    chunkElementCount
  );
  const right = new Int32Array(
    sharedBuffer,
    rightArrayByteOffsetInSAB + chunkStartElementOffset * 4, // Calculate byte offset for the chunk
    chunkElementCount
  );

  // Process the data
  let result = 0;
  // The length of the views (left.length or right.length) is now chunkElementCount
  for (let i = 0; i < chunkElementCount; i++) {
    result += Math.sqrt(Math.abs(Math.cos(left[i]) * Math.sin(right[i])));
  }

  // Send the result back to the main thread
  self.postMessage(result);
}

self.onmessage = handleMessage;
