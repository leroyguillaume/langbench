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

function mergeSort(&$arr, $left, $right) {
    if ($left < $right) {
        $mid = (int)($left + ($right - $left) / 2);
        mergeSort($arr, $left, $mid);
        mergeSort($arr, $mid + 1, $right);
        merge($arr, $left, $mid, $right);
    }
}

function main() {
    $argv = $_SERVER['argv'];
    $argc = count($argv);

    if ($argc != 4) {
        fprintf(STDERR, "Usage: %s <input_file> <num_integers> <output_file>\n", $argv[0]);
        exit(1);
    }

    $input_file = $argv[1];
    $num_integers = (int)$argv[2];
    $output_file = $argv[3];

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

    // Perform merge sort
    mergeSort($arr, 0, $num_integers - 1);

    // Write output file
    $output_content = pack("i*", ...$arr);
    if (@file_put_contents($output_file, $output_content) === false) {
        fprintf(STDERR, "Error writing output file\n");
        exit(1);
    }
}

main();
