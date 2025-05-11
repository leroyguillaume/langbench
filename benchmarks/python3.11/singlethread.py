#!/usr/bin/env python3

import mmap
import math
import sys

def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <filepath> <size>", file=sys.stderr)
        sys.exit(1)

    try:
        size = int(sys.argv[2])
        if size <= 0:
            print("Error: Size must be a positive integer", file=sys.stderr)
            sys.exit(1)
    except ValueError:
        print("Error: Size must be a positive integer", file=sys.stderr)
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

            # Perform the same mathematical operations as the C version
            result = 0.0
            for i in range(half_size):
                result += math.sqrt(abs(math.cos(data[i]) * math.sin(data[half_size + i])))

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
