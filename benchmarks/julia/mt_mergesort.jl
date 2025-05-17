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

function merge_sort_sequential!(arr::Vector{Int32}, left::Int, right::Int)
    if left < right
        mid = div(left + right, 2)
        merge_sort_sequential!(arr, left, mid)
        merge_sort_sequential!(arr, mid + 1, right)
        merge!(arr, left, mid, right)
    end
end

function process_chunk(chunk::Vector{Int32})
    merge_sort_sequential!(chunk, 1, length(chunk))
    return chunk
end

function merge_sort_parallel!(arr::Vector{Int32}, num_workers::Int)
    # Calculate chunk size and create chunks
    chunk_size = div(length(arr), num_workers)
    chunks = Vector{Vector{Int32}}(undef, num_workers)

    for i in 1:num_workers
        start_idx = (i-1) * chunk_size + 1
        end_idx = i == num_workers ? length(arr) : i * chunk_size
        chunks[i] = arr[start_idx:end_idx]
    end

    # Sort chunks in parallel
    @threads for i in 1:num_workers
        chunks[i] = process_chunk(chunks[i])
    end

    # Merge all sorted chunks
    result = Vector{Int32}(undef, length(arr))
    current_pos = 1
    for chunk in chunks
        for i in 1:length(chunk)
            result[current_pos] = chunk[i]
            current_pos += 1
        end
    end

    # Final merge sort on the combined array
    merge_sort_sequential!(result, 1, length(result))
    return result
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

    # Read input file
    arr = Vector{Int32}(undef, num_integers)
    open(input_file, "r") do io
        read!(io, arr)
    end

    # Perform parallel merge sort
    sorted_arr = merge_sort_parallel!(arr, num_cores)

    # Write output file
    open(output_file, "w") do io
        write(io, sorted_arr)
    end
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
