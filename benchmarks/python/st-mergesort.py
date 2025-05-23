#!/usr/bin/env python3

import sys
import array

def merge(arr, left, mid, right):
    n1 = mid - left + 1
    n2 = right - mid

    # Create temporary arrays
    L = array.array('i', [0] * n1)
    R = array.array('i', [0] * n2)

    # Copy data to temporary arrays using memory copy equivalent
    L[:] = arr[left:left + n1]
    R[:] = arr[mid + 1:mid + 1 + n2]

    # Merge the temporary arrays back
    i = j = 0
    k = left
    while i < n1 and j < n2:
        if L[i] <= R[j]:
            arr[k] = L[i]
            i += 1
        else:
            arr[k] = R[j]
            j += 1
        k += 1

    # Copy remaining elements of L[]
    while i < n1:
        arr[k] = L[i]
        i += 1
        k += 1

    # Copy remaining elements of R[]
    while j < n2:
        arr[k] = R[j]
        j += 1
        k += 1

def merge_sort(arr, left, right):
    if left < right:
        mid = left + (right - left) // 2  # Changed to match C version's calculation
        merge_sort(arr, left, mid)
        merge_sort(arr, mid + 1, right)
        merge(arr, left, mid, right)

def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <input_file> <num_integers> <output_file>", file=sys.stderr)
        sys.exit(1)

    input_file = sys.argv[1]
    try:
        num_integers = int(sys.argv[2])
    except ValueError:
        print("Error: num_integers must be a valid integer", file=sys.stderr)
        sys.exit(1)
    output_file = sys.argv[3]

    # Read input file
    try:
        with open(input_file, 'rb') as f:
            arr = array.array('i')
            try:
                arr.fromfile(f, num_integers)
            except EOFError:
                print("Error: Input file is too short", file=sys.stderr)
                sys.exit(1)
    except IOError as e:
        print(f"Error opening input file: {e}", file=sys.stderr)
        sys.exit(1)

    # Perform merge sort
    merge_sort(arr, 0, num_integers - 1)

    # Write output file
    try:
        with open(output_file, 'wb') as f:
            arr.tofile(f)
    except IOError as e:
        print(f"Error writing output file: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
