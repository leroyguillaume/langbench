#!/usr/bin/env julia

using Base.Threads

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

function merge_sort_parallel!(arr::Vector{Int32}, left::Int, right::Int, depth::Int, max_depth::Int)
    if left < right
        mid = div(left + right, 2)

        if depth < max_depth
            # Create threads for left and right halves
            left_task = Threads.@spawn merge_sort_parallel!(arr, left, mid, depth + 1, max_depth)
            right_task = Threads.@spawn merge_sort_parallel!(arr, mid + 1, right, depth + 1, max_depth)

            # Wait for both tasks to complete
            wait(left_task)
            wait(right_task)
        else
            # Sequential sorting for remaining depth
            merge_sort_parallel!(arr, left, mid, depth + 1, max_depth)
            merge_sort_parallel!(arr, mid + 1, right, depth + 1, max_depth)
        end

        merge!(arr, left, mid, right)
    end
end

function main()
    if length(ARGS) != 4
        println("Usage: julia mt_mergesort.jl <input_file> <num_integers> <num_cores> <output_file>")
        exit(1)
    end

    input_file = ARGS[1]
    num_integers = parse(Int, ARGS[2])
    num_cores = parse(Int, ARGS[3])
    output_file = ARGS[4]

    # Set number of threads
    Threads.nthreads() != num_cores && @warn "Requested $num_cores threads but got $(Threads.nthreads())"

    # Calculate max depth for thread creation
    max_depth = 0
    temp = num_cores
    while temp > 1
        max_depth += 1
        temp ÷= 2
    end

    # Read input file
    arr = Vector{Int32}(undef, num_integers)
    open(input_file, "r") do io
        read!(io, arr)
    end

    # Perform parallel merge sort
    merge_sort_parallel!(arr, 1, length(arr), 0, max_depth)

    # Write output file
    open(output_file, "w") do io
        write(io, arr)
    end
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
