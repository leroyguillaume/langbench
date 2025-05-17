#!/usr/bin/env php
<?php

function merge(&$arr, $left, $mid, $right) {
    $n1 = $mid - $left + 1;
    $n2 = $right - $mid;

    // Create temporary arrays
    $L = array_fill(0, $n1, 0);
    $R = array_fill(0, $n2, 0);

    // Copy data to temporary arrays
    for ($i = 0; $i < $n1; $i++) {
        $L[$i] = $arr[$left + $i];
    }
    for ($j = 0; $j < $n2; $j++) {
        $R[$j] = $arr[$mid + 1 + $j];
    }

    // Merge the temporary arrays back
    $i = $j = 0;
    $k = $left;
    while ($i < $n1 && $j < $n2) {
        if ($L[$i] <= $R[$j]) {
            $arr[$k] = $L[$i];
            $i++;
        } else {
            $arr[$k] = $R[$j];
            $j++;
        }
        $k++;
    }

    // Copy remaining elements of L[]
    while ($i < $n1) {
        $arr[$k] = $L[$i];
        $i++;
        $k++;
    }

    // Copy remaining elements of R[]
    while ($j < $n2) {
        $arr[$k] = $R[$j];
        $j++;
        $k++;
    }
}

function merge_sort_sequential(&$arr, $left, $right) {
    if ($left < $right) {
        $mid = (int)(($left + $right) / 2);
        merge_sort_sequential($arr, $left, $mid);
        merge_sort_sequential($arr, $mid + 1, $right);
        merge($arr, $left, $mid, $right);
    }
}

function process_chunk($chunk) {
    merge_sort_sequential($chunk, 0, count($chunk) - 1);
    return $chunk;
}

function merge_sort_parallel($arr, $num_workers) {
    // Calculate chunk size and distribute tasks
    $chunk_size = (int)(count($arr) / $num_workers);
    $chunks = [];

    for ($i = 0; $i < $num_workers; $i++) {
        $start = $i * $chunk_size;
        $end = ($i < $num_workers - 1) ? $start + $chunk_size : count($arr);
        $chunks[] = array_slice($arr, $start, $end - $start);
    }

    // Process chunks in parallel using pcntl_fork
    $pids = [];
    $temp_files = [];

    foreach ($chunks as $i => $chunk) {
        $temp_file = tempnam(sys_get_temp_dir(), 'sort_');
        $temp_files[] = $temp_file;

        $pid = pcntl_fork();
        if ($pid == -1) {
            die('Could not fork');
        } else if ($pid) {
            // Parent process
            $pids[] = $pid;
        } else {
            // Child process
            $sorted_chunk = process_chunk($chunk);
            file_put_contents($temp_file, serialize($sorted_chunk));
            exit(0);
        }
    }

    // Wait for all child processes to complete
    foreach ($pids as $pid) {
        pcntl_waitpid($pid, $status);
    }

    // Collect results from temp files
    $sorted_chunks = [];
    foreach ($temp_files as $temp_file) {
        $sorted_chunks[] = unserialize(file_get_contents($temp_file));
        unlink($temp_file);
    }

    // Merge all sorted chunks
    $result = array_fill(0, count($arr), 0);
    $current_pos = 0;
    foreach ($sorted_chunks as $chunk) {
        foreach ($chunk as $value) {
            $result[$current_pos] = $value;
            $current_pos++;
        }
    }

    // Final merge sort on the combined array
    merge_sort_sequential($result, 0, count($result) - 1);
    return $result;
}

function main() {
    if (!function_exists('pcntl_fork')) {
        echo "Error: pcntl extension is required for parallel processing\n";
        exit(1);
    }

    $argv = $_SERVER['argv'];
    $argc = count($argv);

    if ($argc != 5) {
        echo "Usage: php mt-mergesort.php <input_file> <num_integers> <num_cores> <output_file>\n";
        exit(1);
    }

    $input_file = $argv[1];
    $num_integers = (int)$argv[2];
    $num_cores = (int)$argv[3];
    $output_file = $argv[4];

    // Read input file
    $arr = array_values(unpack("i*", file_get_contents($input_file)));
    if (count($arr) < $num_integers) {
        echo "Error: Input file contains fewer integers than specified\n";
        exit(1);
    }
    $arr = array_slice($arr, 0, $num_integers);

    // Perform parallel merge sort
    $sorted_arr = merge_sort_parallel($arr, $num_cores);

    // Write output file
    file_put_contents($output_file, pack("i*", ...$sorted_arr));
}

main();
