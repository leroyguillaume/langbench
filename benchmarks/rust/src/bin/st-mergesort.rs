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
        let mid = left + (right - left) / 2;
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
    let num_integers: usize = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Failed to parse number of integers");
        std::process::exit(1);
    });
    let output_file = &args[3];

    // Read input file
    let mut file = match File::open(input_file) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error opening input file");
            std::process::exit(1);
        }
    };

    let mut buffer = vec![0i32; num_integers];
    let buffer_bytes = unsafe {
        std::slice::from_raw_parts_mut(
            buffer.as_mut_ptr() as *mut u8,
            num_integers * std::mem::size_of::<i32>()
        )
    };
    if file.read_exact(buffer_bytes).is_err() {
        eprintln!("Error reading input file");
        std::process::exit(1);
    }

    // Perform merge sort
    merge_sort(&mut buffer, 0, num_integers - 1);

    // Write output file
    let mut output = match File::create(output_file) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error opening output file");
            std::process::exit(1);
        }
    };

    let buffer_bytes = unsafe {
        std::slice::from_raw_parts(
            buffer.as_ptr() as *const u8,
            num_integers * std::mem::size_of::<i32>()
        )
    };
    if output.write_all(buffer_bytes).is_err() {
        eprintln!("Error writing output file");
        std::process::exit(1);
    }
}
