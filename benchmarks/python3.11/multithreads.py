#!/usr/bin/env python3

import mmap
import math
import sys
import threading

class Chunk:
    def __init__(self, left, right, size):
        self.left = left
        self.right = right
        self.size = size
        self.result = 0.0

def compute(chunk):
    for i in range(chunk.size):
        chunk.result += math.sqrt(abs(math.cos(chunk.left[i]) * math.sin(chunk.right[i])))

def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <filepath> <size> <threads>", file=sys.stderr)
        sys.exit(1)

    try:
        size = int(sys.argv[2])
        if size <= 0:
            print("Error: Size must be a positive integer", file=sys.stderr)
            sys.exit(1)
    except ValueError:
        print("Error: Size must be a positive integer", file=sys.stderr)
        sys.exit(1)

    try:
        num_threads = int(sys.argv[3])
        if num_threads <= 0:
            print("Error: Threads must be a positive integer", file=sys.stderr)
            sys.exit(1)
    except ValueError:
        print("Error: Threads must be a positive integer", file=sys.stderr)
        sys.exit(1)

    half_size = size // 2

    try:
        with open(sys.argv[1], 'rb') as f:
            # Memory map the file
            mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)

            # Convert the memory mapped data to integers
            data = []
            for i in range(size):
                data.append(int.from_bytes(mm.read(4), byteorder='little'))

            # Calculate chunk sizes
            chunk_size = half_size // num_threads
            chunk_size_overflow = half_size % num_threads

            # Create and start threads
            threads = []
            chunks = []
            current_pos = 0

            for i in range(num_threads):
                current_chunk_size = chunk_size + (1 if i < chunk_size_overflow else 0)
                chunk = Chunk(
                    data[current_pos:current_pos + current_chunk_size],
                    data[half_size + current_pos:half_size + current_pos + current_chunk_size],
                    current_chunk_size
                )
                chunks.append(chunk)
                thread = threading.Thread(target=compute, args=(chunk,))
                threads.append(thread)
                thread.start()
                current_pos += current_chunk_size

            # Wait for all threads to complete
            for thread in threads:
                thread.join()

            # Sum up results
            result = sum(chunk.result for chunk in chunks)
            print(f"{result}")

            mm.close()

    except FileNotFoundError:
        print(f"Error: Could not open file {sys.argv[1]}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {str(e)}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
