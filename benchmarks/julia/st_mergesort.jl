#!/usr/bin/env julia

function merge!(arr::Vector{Int32}, left::Int, mid::Int, right::Int)
    n1 = mid - left + 1
    n2 = right - mid

    # Create temporary arrays
    L = Vector{Int32}(undef, n1)
    R = Vector{Int32}(undef, n2)

    # Copy data to temporary arrays
    for i in 1:n1
        L[i] = arr[left + i - 1]
    end
    for j in 1:n2
        R[j] = arr[mid + j]
    end

    # Merge the temporary arrays back
    i = j = 1
    k = left
    while i ≤ n1 && j ≤ n2
        if L[i] ≤ R[j]
            arr[k] = L[i]
            i += 1
        else
            arr[k] = R[j]
            j += 1
        end
        k += 1
    end

    # Copy remaining elements of L[]
    while i ≤ n1
        arr[k] = L[i]
        i += 1
        k += 1
    end

    # Copy remaining elements of R[]
    while j ≤ n2
        arr[k] = R[j]
        j += 1
        k += 1
    end
end

function merge_sort!(arr::Vector{Int32}, left::Int, right::Int)
    if left < right
        mid = div(left + right, 2)
        merge_sort!(arr, left, mid)
        merge_sort!(arr, mid + 1, right)
        merge!(arr, left, mid, right)
    end
end

function main()
    if length(ARGS) != 3
        println("Usage: julia st_mergesort.jl <input_file> <num_integers> <output_file>")
        exit(1)
    end

    input_file = ARGS[1]
    num_integers = parse(Int, ARGS[2])
    output_file = ARGS[3]

    # Read input file
    arr = Vector{Int32}(undef, num_integers)
    open(input_file, "r") do io
        read!(io, arr)
    end

    # Perform merge sort
    merge_sort!(arr, 1, num_integers)

    # Write output file
    open(output_file, "w") do io
        write(io, arr)
    end
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
