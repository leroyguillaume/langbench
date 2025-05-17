use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::thread;

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

fn parallel_merge_sort(arr: &mut [i32], num_threads: usize) {
    let len = arr.len();
    if len <= 1 {
        return;
    }

    // For small arrays or when we've reached the thread limit, use sequential sort
    if len <= 1000 || num_threads <= 1 {
        merge_sort(arr, 0, len - 1);
        return;
    }

    let mid = len / 2;

    // Create a copy of the right half for the thread
    let mut right_half = arr[mid..].to_vec();

    // Spawn a thread to sort the right half
    let right_handle = thread::spawn(move || {
        parallel_merge_sort(&mut right_half, num_threads / 2);
        right_half
    });

    // Sort the left half in the current thread
    parallel_merge_sort(&mut arr[..mid], num_threads / 2);

    // Wait for the right half to complete and merge the results
    let right_sorted = right_handle.join().unwrap();
    arr[mid..].copy_from_slice(&right_sorted);
    merge(arr, 0, mid - 1, len - 1);
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
    let mut args = env::args();
    let program_name = args.next().unwrap();

    let input_file = args.next().unwrap_or_else(|| {
        eprintln!("Usage: {} <input_file> <num_integers> <num_threads> <output_file>", program_name);
        std::process::exit(1);
    });

    let num_integers: usize = args.next()
        .and_then(|arg| arg.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Failed to parse number of integers");
            std::process::exit(1);
        });

    let num_threads: usize = args.next()
        .and_then(|arg| arg.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Failed to parse number of threads");
            std::process::exit(1);
        });

    let output_file = args.next().unwrap_or_else(|| {
        eprintln!("Usage: {} <input_file> <num_integers> <num_threads> <output_file>", program_name);
        std::process::exit(1);
    });

    if args.next().is_some() {
        eprintln!("Usage: {} <input_file> <num_integers> <num_threads> <output_file>", program_name);
        std::process::exit(1);
    }

    // Read input file
    let mut file = File::open(input_file).expect("Failed to open input file");
    let mut buffer = vec![0i32; num_integers];
    file.read_exact(unsafe {
        std::slice::from_raw_parts_mut(
            buffer.as_mut_ptr() as *mut u8,
            num_integers * std::mem::size_of::<i32>()
        )
    }).expect("Failed to read input file");

    // Perform parallel merge sort
    parallel_merge_sort(&mut buffer, num_threads);

    // Write output file
    let mut output = File::create(output_file).expect("Failed to create output file");
    output.write_all(unsafe {
        std::slice::from_raw_parts(
            buffer.as_ptr() as *const u8,
            num_integers * std::mem::size_of::<i32>()
        )
    }).expect("Failed to write output file");
}
