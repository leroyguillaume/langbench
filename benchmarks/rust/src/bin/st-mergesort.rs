use std::env;
use std::fs::File;
use std::io::{Read, Write};

fn merge(arr: &mut [i32], left: usize, mid: usize, right: usize) {
    let n1 = mid - left + 1;
    let n2 = right - mid;

    // Create temporary arrays
    let mut left_arr = vec![0; n1];
    let mut right_arr = vec![0; n2];

    // Copy data to temporary arrays
    left_arr.copy_from_slice(&arr[left..=mid]);
    right_arr.copy_from_slice(&arr[mid + 1..=right]);

    // Merge the temporary arrays back
    let mut i = 0;
    let mut j = 0;
    let mut k = left;

    while i < n1 && j < n2 {
        if left_arr[i] <= right_arr[j] {
            arr[k] = left_arr[i];
            i += 1;
        } else {
            arr[k] = right_arr[j];
            j += 1;
        }
        k += 1;
    }

    // Copy remaining elements of left_arr
    while i < n1 {
        arr[k] = left_arr[i];
        i += 1;
        k += 1;
    }

    // Copy remaining elements of right_arr
    while j < n2 {
        arr[k] = right_arr[j];
        j += 1;
        k += 1;
    }
}

fn merge_sort(arr: &mut [i32], left: usize, right: usize) {
    if left < right {
        let mid = (left + right) / 2;
        merge_sort(arr, left, mid);
        merge_sort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <input_file> <num_integers> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let num_integers: usize = args[2].parse().expect("Failed to parse number of integers");
    let output_file = &args[3];

    // Read input file
    let mut file = File::open(input_file).expect("Failed to open input file");
    let mut buffer = vec![0i32; num_integers];
    let bytes_to_read = num_integers * std::mem::size_of::<i32>();
    let mut bytes = vec![0u8; bytes_to_read];
    file.read_exact(&mut bytes).expect("Failed to read input file");

    // Convert bytes to i32 array
    for i in 0..num_integers {
        buffer[i] = i32::from_le_bytes([
            bytes[i * 4],
            bytes[i * 4 + 1],
            bytes[i * 4 + 2],
            bytes[i * 4 + 3],
        ]);
    }

    // Perform merge sort
    merge_sort(&mut buffer, 0, num_integers - 1);

    // Write output file
    let mut output = File::create(output_file).expect("Failed to create output file");
    let bytes: Vec<u8> = buffer.iter()
        .flat_map(|&x| x.to_le_bytes().to_vec())
        .collect();
    output.write_all(&bytes).expect("Failed to write output file");
}
