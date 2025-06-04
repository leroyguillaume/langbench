use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::thread;
use std::sync::{Arc, Mutex};

// Structure to pass arguments to the thread function
struct ThreadArgs {
    arr: Arc<Mutex<Vec<i32>>>,
    left: usize,
    right: usize,
    depth: usize,
    max_depth: usize,
}

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

fn merge_sort_thread(args: ThreadArgs) {
    let left = args.left;
    let right = args.right;
    let depth = args.depth;
    let max_depth = args.max_depth;

    if left < right {
        let mid = left + (right - left) / 2;

        if depth < max_depth {
            // Create threads for left and right halves
            let left_args = ThreadArgs {
                arr: Arc::clone(&args.arr),
                left,
                right: mid,
                depth: depth + 1,
                max_depth,
            };

            let right_args = ThreadArgs {
                arr: Arc::clone(&args.arr),
                left: mid + 1,
                right,
                depth: depth + 1,
                max_depth,
            };

            let left_handle = thread::spawn(move || merge_sort_thread(left_args));
            let right_handle = thread::spawn(move || merge_sort_thread(right_args));

            left_handle.join().unwrap();
            right_handle.join().unwrap();
        } else {
            // Sequential sorting for remaining depth
            let left_args = ThreadArgs {
                arr: Arc::clone(&args.arr),
                left,
                right: mid,
                depth: depth + 1,
                max_depth,
            };

            let right_args = ThreadArgs {
                arr: Arc::clone(&args.arr),
                left: mid + 1,
                right,
                depth: depth + 1,
                max_depth,
            };

            merge_sort_thread(left_args);
            merge_sort_thread(right_args);
        }

        // Merge the sorted halves
        let mut arr = args.arr.lock().unwrap();
        merge(&mut arr, left, mid, right);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage: {} <input_file> <num_integers> <num_cores> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let num_integers: usize = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Failed to parse number of integers");
        std::process::exit(1);
    });
    let num_cores: usize = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Failed to parse number of cores");
        std::process::exit(1);
    });
    let output_file = &args[4];

    // Calculate max depth for thread creation
    let mut max_depth = 0;
    let mut temp = num_cores;
    while temp > 1 {
        max_depth += 1;
        temp /= 2;
    }

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

    // Create shared array
    let shared_arr = Arc::new(Mutex::new(buffer));

    // Perform parallel merge sort
    let args = ThreadArgs {
        arr: shared_arr.clone(),
        left: 0,
        right: num_integers - 1,
        depth: 0,
        max_depth,
    };
    merge_sort_thread(args);

    // Get the sorted array
    let buffer = Arc::try_unwrap(shared_arr).unwrap().into_inner().unwrap();

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
