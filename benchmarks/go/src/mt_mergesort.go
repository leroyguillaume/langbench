package main

import (
	"flag"
	"fmt"
	"os"
	"sync"
	"unsafe"
)

// ThreadArgs holds the arguments for a merge sort operation
type ThreadArgs struct {
	arr       []int32
	left      int
	right     int
	depth     int
	maxDepth  int
}

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

// mergeSortThread performs parallel merge sort using goroutines
func mergeSortThread(args *ThreadArgs) {
	arr := args.arr
	left := args.left
	right := args.right
	depth := args.depth
	maxDepth := args.maxDepth

	if left < right {
		mid := left + (right-left)/2

		if depth < maxDepth {
			// Create goroutines for left and right halves
			var wg sync.WaitGroup
			wg.Add(2)

			// Process left half
			go func() {
				defer wg.Done()
				leftArgs := &ThreadArgs{
					arr:      arr,
					left:     left,
					right:    mid,
					depth:    depth + 1,
					maxDepth: maxDepth,
				}
				mergeSortThread(leftArgs)
			}()

			// Process right half
			go func() {
				defer wg.Done()
				rightArgs := &ThreadArgs{
					arr:      arr,
					left:     mid + 1,
					right:    right,
					depth:    depth + 1,
					maxDepth: maxDepth,
				}
				mergeSortThread(rightArgs)
			}()

			wg.Wait()
		} else {
			// Sequential sorting for remaining depth
			leftArgs := &ThreadArgs{
				arr:      arr,
				left:     left,
				right:    mid,
				depth:    depth + 1,
				maxDepth: maxDepth,
			}
			rightArgs := &ThreadArgs{
				arr:      arr,
				left:     mid + 1,
				right:    right,
				depth:    depth + 1,
				maxDepth: maxDepth,
			}
			mergeSortThread(leftArgs)
			mergeSortThread(rightArgs)
		}

		merge(arr, left, mid, right)
	}
}

func main() {
	// Parse command line arguments
	flag.Parse()
	args := flag.Args()
	if len(args) != 4 {
		fmt.Fprintf(os.Stderr, "Usage: %s <input_file> <num_integers> <num_cores> <output_file>\n", os.Args[0])
		os.Exit(1)
	}

	inputFile := args[0]
	numIntegers := parseInt(args[1])
	numCores := parseInt(args[2])
	outputFile := args[3]

	// Calculate max depth for goroutine creation
	maxDepth := 0
	temp := numCores
	for temp > 1 {
		maxDepth++
		temp /= 2
	}

	// Allocate memory for the array
	arr := make([]int32, numIntegers)

	// Read input file
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
	threadArgs := &ThreadArgs{
		arr:      arr,
		left:     0,
		right:    len(arr) - 1,
		depth:    0,
		maxDepth: maxDepth,
	}
	mergeSortThread(threadArgs)

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
