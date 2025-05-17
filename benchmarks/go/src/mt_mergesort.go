package main

import (
	"flag"
	"fmt"
	"os"
	"sync"
	"unsafe"
)

// merge combines two sorted subarrays into a single sorted array
func merge(arr []int32, left, mid, right int) {
	n1 := mid - left + 1
	n2 := right - mid

	// Create temporary arrays
	L := make([]int32, n1)
	R := make([]int32, n2)

	// Copy data to temporary arrays
	copy(L, arr[left:left+n1])
	copy(R, arr[mid+1:mid+1+n2])

	// Merge the temporary arrays back
	i, j, k := 0, 0, left
	for i < n1 && j < n2 {
		if L[i] <= R[j] {
			arr[k] = L[i]
			i++
		} else {
			arr[k] = R[j]
			j++
		}
		k++
	}

	// Copy remaining elements of L[]
	for i < n1 {
		arr[k] = L[i]
		i++
		k++
	}

	// Copy remaining elements of R[]
	for j < n2 {
		arr[k] = R[j]
		j++
		k++
	}
}

// mergeSortSequential performs sequential merge sort
func mergeSortSequential(arr []int32, left, right int) {
	if left < right {
		mid := (left + right) / 2
		mergeSortSequential(arr, left, mid)
		mergeSortSequential(arr, mid+1, right)
		merge(arr, left, mid, right)
	}
}

// processChunk sorts a chunk of the array
func processChunk(chunk []int32) []int32 {
	mergeSortSequential(chunk, 0, len(chunk)-1)
	return chunk
}

// mergeSortParallel performs parallel merge sort using goroutines
func mergeSortParallel(arr []int32, numWorkers int) []int32 {
	// Create channels for task distribution and results
	tasks := make(chan []int32, numWorkers)
	results := make(chan []int32, numWorkers)
	var wg sync.WaitGroup

	// Start worker goroutines
	for i := 0; i < numWorkers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for chunk := range tasks {
				results <- processChunk(chunk)
			}
		}()
	}

	// Calculate chunk size and distribute tasks
	chunkSize := len(arr) / numWorkers
	for i := 0; i < numWorkers; i++ {
		start := i * chunkSize
		end := start + chunkSize
		if i == numWorkers-1 {
			end = len(arr)
		}
		tasks <- arr[start:end]
	}
	close(tasks)

	// Wait for all workers to finish
	go func() {
		wg.Wait()
		close(results)
	}()

	// Collect results
	sortedChunks := make([][]int32, 0, numWorkers)
	for chunk := range results {
		sortedChunks = append(sortedChunks, chunk)
	}

	// Combine all sorted chunks
	result := make([]int32, len(arr))
	currentPos := 0
	for _, chunk := range sortedChunks {
		copy(result[currentPos:], chunk)
		currentPos += len(chunk)
	}

	// Final merge sort on the combined array
	mergeSortSequential(result, 0, len(result)-1)
	return result
}

func main() {
	// Parse command line arguments
	flag.Parse()
	args := flag.Args()
	if len(args) != 4 {
		fmt.Println("Usage: go run mt-mergesort.go <input_file> <num_integers> <num_cores> <output_file>")
		os.Exit(1)
	}

	inputFile := args[0]
	numIntegers := parseInt(args[1])
	numCores := parseInt(args[2])
	outputFile := args[3]

	// Read input file
	arr := make([]int32, numIntegers)
	file, err := os.Open(inputFile)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening input file: %v\n", err)
		os.Exit(1)
	}
	defer file.Close()

	// Read all integers at once
	_, err = file.Read(unsafe.Slice((*byte)(unsafe.Pointer(&arr[0])), numIntegers*4))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading input file: %v\n", err)
		os.Exit(1)
	}

	// Perform parallel merge sort
	sortedArr := mergeSortParallel(arr, numCores)

	// Write output file
	outFile, err := os.Create(outputFile)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating output file: %v\n", err)
		os.Exit(1)
	}
	defer outFile.Close()

	// Write all integers at once
	_, err = outFile.Write(unsafe.Slice((*byte)(unsafe.Pointer(&sortedArr[0])), len(sortedArr)*4))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error writing output file: %v\n", err)
		os.Exit(1)
	}
}

func parseInt(s string) int {
	var i int
	_, err := fmt.Sscanf(s, "%d", &i)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error parsing integer: %v\n", err)
		os.Exit(1)
	}
	return i
}
