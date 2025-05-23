#!/usr/bin/env php
<?php

function merge($shm_id, $left, $mid, $right) {
    $n1 = $mid - $left + 1;
    $n2 = $right - $mid;

    // Create temporary arrays
    $L = array_fill(0, $n1, 0);
    $R = array_fill(0, $n2, 0);

    // Copy data to temporary arrays
    for ($i = 0; $i < $n1; $i++) {
        $L[$i] = shmop_read($shm_id, ($left + $i) * 4, 4);
        $L[$i] = unpack("i", $L[$i])[1];
    }
    for ($j = 0; $j < $n2; $j++) {
        $R[$j] = shmop_read($shm_id, ($mid + 1 + $j) * 4, 4);
        $R[$j] = unpack("i", $R[$j])[1];
    }

    // Merge the temporary arrays back
    $i = $j = 0;
    $k = $left;
    while ($i < $n1 && $j < $n2) {
        if ($L[$i] <= $R[$j]) {
            shmop_write($shm_id, pack("i", $L[$i]), $k * 4);
            $i++;
        } else {
            shmop_write($shm_id, pack("i", $R[$j]), $k * 4);
            $j++;
        }
        $k++;
    }

    // Copy remaining elements of L[]
    while ($i < $n1) {
        shmop_write($shm_id, pack("i", $L[$i]), $k * 4);
        $i++;
        $k++;
    }

    // Copy remaining elements of R[]
    while ($j < $n2) {
        shmop_write($shm_id, pack("i", $R[$j]), $k * 4);
        $j++;
        $k++;
    }
}

function mergeSortThread($shm_id, $left, $right, $depth, $max_depth) {
    if ($left < $right) {
        $mid = (int)($left + ($right - $left) / 2);

        if ($depth < $max_depth) {
            // Create child process for left half
            $pid = pcntl_fork();
            if ($pid == -1) {
                die('Could not fork');
            } else if ($pid) {
                // Parent process - handle right half
                mergeSortThread($shm_id, $mid + 1, $right, $depth + 1, $max_depth);
                pcntl_waitpid($pid, $status);
            } else {
                // Child process - handle left half
                mergeSortThread($shm_id, $left, $mid, $depth + 1, $max_depth);
                exit(0);
            }
        } else {
            // Sequential sorting for remaining depth
            mergeSortThread($shm_id, $left, $mid, $depth + 1, $max_depth);
            mergeSortThread($shm_id, $mid + 1, $right, $depth + 1, $max_depth);
        }

        merge($shm_id, $left, $mid, $right);
    }
}

function main() {
    if (!function_exists('pcntl_fork')) {
        fprintf(STDERR, "Error: pcntl extension is required for parallel processing\n");
        exit(1);
    }

    if (!function_exists('shmop_open')) {
        fprintf(STDERR, "Error: shmop extension is required for shared memory\n");
        exit(1);
    }

    $argv = $_SERVER['argv'];
    $argc = count($argv);

    if ($argc != 5) {
        fprintf(STDERR, "Usage: %s <input_file> <num_integers> <num_cores> <output_file>\n", $argv[0]);
        exit(1);
    }

    $input_file = $argv[1];
    $num_integers = (int)$argv[2];
    $num_cores = (int)$argv[3];
    $output_file = $argv[4];

    // Calculate max depth for process creation
    $max_depth = 0;
    $temp = $num_cores;
    while ($temp > 1) {
        $max_depth++;
        $temp /= 2;
    }

    // Read input file
    $input_content = @file_get_contents($input_file);
    if ($input_content === false) {
        fprintf(STDERR, "Error opening input file\n");
        exit(1);
    }

    $arr = array_values(unpack("i*", $input_content));
    if (count($arr) < $num_integers) {
        fprintf(STDERR, "Error: Input file contains fewer integers than specified\n");
        exit(1);
    }
    $arr = array_slice($arr, 0, $num_integers);

    // Create shared memory segment
    $shm_size = $num_integers * 4; // 4 bytes per integer
    $shm_id = shmop_open(ftok(__FILE__, 't'), "c", 0644, $shm_size);
    if ($shm_id === false) {
        fprintf(STDERR, "Error creating shared memory segment\n");
        exit(1);
    }

    // Copy array to shared memory
    for ($i = 0; $i < $num_integers; $i++) {
        shmop_write($shm_id, pack("i", $arr[$i]), $i * 4);
    }

    // Perform parallel merge sort
    mergeSortThread($shm_id, 0, $num_integers - 1, 0, $max_depth);

    // Read result from shared memory
    $result = array_fill(0, $num_integers, 0);
    for ($i = 0; $i < $num_integers; $i++) {
        $value = shmop_read($shm_id, $i * 4, 4);
        $result[$i] = unpack("i", $value)[1];
    }

    // Clean up shared memory
    shmop_delete($shm_id);
    shmop_close($shm_id);

    // Write output file
    $output_content = pack("i*", ...$result);
    if (@file_put_contents($output_file, $output_content) === false) {
        fprintf(STDERR, "Error writing output file\n");
        exit(1);
    }
}

main();
