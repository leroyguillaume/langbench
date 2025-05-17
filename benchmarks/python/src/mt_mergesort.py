#!/usr/bin/env python3

import sys
import array
import threading
from interpreters_backport import interpreters

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

def merge_sort_sequential(arr, left, right):
    if left < right:
        mid = (left + right) // 2
        merge_sort_sequential(arr, left, mid)
        merge_sort_sequential(arr, mid + 1, right)
        merge(arr, left, mid, right)

def worker(tasks, results):
    interp = interpreters.create()
    interp.exec("import interpreters_backport.interpreters.queues")
    interp.prepare_main(tasks=tasks, results=results)

    # Define the worker code as a string
    worker_code = """
import array
from mt_mergesort import merge, merge_sort_sequential

def process_chunk(chunk):
    arr = array.array('i', chunk)
    merge_sort_sequential(arr, 0, len(arr) - 1)
    return arr

while True:
    task = tasks.get()
    if task is None:
        break
    chunk = task
    result = process_chunk(chunk)
    results.put(result)
"""
    interp.exec(worker_code)

def merge_sort_parallel(arr, num_workers):
    # Create queues for task distribution and results
    tasks = interpreters.create_queue()
    results = interpreters.create_queue()

    # Start worker threads
    threads = []
    for _ in range(num_workers):
        t = threading.Thread(target=worker, args=(tasks, results))
        t.daemon = True  # Make threads daemon so they exit when main thread exits
        t.start()
        threads.append(t)

    # Calculate chunk size and distribute tasks
    chunk_size = len(arr) // num_workers
    for i in range(num_workers):
        start = i * chunk_size
        end = start + chunk_size if i < num_workers - 1 else len(arr)
        chunk = arr[start:end]
        tasks.put(chunk)

    # Collect results
    sorted_chunks = []
    for _ in range(num_workers):
        try:
            chunk = results.get()
            sorted_chunks.append(chunk)
        except Exception as e:
            print(f"Error collecting results: {e}", file=sys.stderr)
            sys.exit(1)

    # Wait for all threads to finish
    for t in threads:
        t.join(timeout=1.0)

    # Merge all sorted chunks
    result = array.array('i', [0] * len(arr))
    current_pos = 0
    for chunk in sorted_chunks:
        for i in range(len(chunk)):
            result[current_pos] = chunk[i]
            current_pos += 1

    # Final merge sort on the combined array
    merge_sort_sequential(result, 0, len(result) - 1)
    return result

def main():
    if len(sys.argv) != 5:
        print("Usage: python mt-mergesort.py <input_file> <num_integers> <num_cores> <output_file>")
        sys.exit(1)

    input_file = sys.argv[1]
    num_integers = int(sys.argv[2])
    num_cores = int(sys.argv[3])
    output_file = sys.argv[4]

    # Read input file
    with open(input_file, 'rb') as f:
        arr = array.array('i')
        arr.fromfile(f, num_integers)

    # Perform parallel merge sort
    sorted_arr = merge_sort_parallel(arr, num_cores)

    # Write output file
    with open(output_file, 'wb') as f:
        sorted_arr.tofile(f)

if __name__ == "__main__":
    main()
