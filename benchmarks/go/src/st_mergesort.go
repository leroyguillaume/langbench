package main

import (
	"flag"
	"fmt"
	"os"
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

// mergeSort performs sequential merge sort
func mergeSort(arr []int32, left, right int) {
	if left < right {
		mid := (left + right) / 2
		mergeSort(arr, left, mid)
		mergeSort(arr, mid+1, right)
		merge(arr, left, mid, right)
	}
}

func main() {
	// Parse command line arguments
	flag.Parse()
	args := flag.Args()
	if len(args) != 3 {
		fmt.Println("Usage: go run st-mergesort.go <input_file> <num_integers> <output_file>")
		os.Exit(1)
	}

	inputFile := args[0]
	numIntegers := parseInt(args[1])
	outputFile := args[2]

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

	// Perform sequential merge sort
	mergeSort(arr, 0, len(arr)-1)

	// Write output file
	outFile, err := os.Create(outputFile)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating output file: %v\n", err)
		os.Exit(1)
	}
	defer outFile.Close()

	// Write all integers at once
	_, err = outFile.Write(unsafe.Slice((*byte)(unsafe.Pointer(&arr[0])), len(arr)*4))
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
