#!/usr/bin/env python3

import sys
import array

def merge(arr, left, mid, right):
    n1 = mid - left + 1
    n2 = right - mid

    # Create temporary arrays
    L = array.array('i', [0] * n1)
    R = array.array('i', [0] * n2)

    # Copy data to temporary arrays
    for i in range(n1):
        L[i] = arr[left + i]
    for j in range(n2):
        R[j] = arr[mid + 1 + j]

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
        mid = (left + right) // 2
        merge_sort(arr, left, mid)
        merge_sort(arr, mid + 1, right)
        merge(arr, left, mid, right)

def main():
    if len(sys.argv) != 4:
        print("Usage: python st-mergesort.py <input_file> <num_integers> <output_file>")
        sys.exit(1)

    input_file = sys.argv[1]
    num_integers = int(sys.argv[2])
    output_file = sys.argv[3]

    # Read input file
    with open(input_file, 'rb') as f:
        arr = array.array('i')
        arr.fromfile(f, num_integers)

    # Perform merge sort
    merge_sort(arr, 0, num_integers - 1)

    # Write output file
    with open(output_file, 'wb') as f:
        arr.tofile(f)

if __name__ == "__main__":
    main()
