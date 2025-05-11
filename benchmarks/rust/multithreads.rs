use std::env;
use std::fs::File;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::thread;
use std::os::unix::io::AsRawFd;
use std::ptr;

#[link(name = "c")]
extern "C" {
    fn mmap(
        addr: *mut u8,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut u8;
    fn munmap(addr: *mut u8, length: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const MAP_PRIVATE: i32 = 0x2;
const MAP_FAILED: *mut u8 = !0 as *mut u8;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Usage: {} <filepath> <size> <threads>", args[0]),
        ));
    }

    let size: usize = args[2]
        .parse()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Size must be a positive integer"))?;
    
    if size == 0 {
        return Err(Error::new(ErrorKind::InvalidInput, "Size must be a positive integer"));
    }

    let num_threads: usize = args[3]
        .parse()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Threads must be a positive integer"))?;
    
    if num_threads == 0 {
        return Err(Error::new(ErrorKind::InvalidInput, "Threads must be a positive integer"));
    }

    let half_size = size / 2;

    let file = File::open(&args[1])?;
    let fd = file.as_raw_fd();
    
    // Memory map the file
    let buffer = unsafe {
        let ptr = mmap(
            ptr::null_mut(),
            size * std::mem::size_of::<i32>(),
            PROT_READ,
            MAP_PRIVATE,
            fd,
            0,
        );
        
        if ptr == MAP_FAILED {
            return Err(Error::last_os_error());
        }
        
        std::slice::from_raw_parts(ptr as *const i32, size)
    };

    let chunk_size = half_size / num_threads;
    let chunk_size_overflow = half_size % num_threads;

    let mut handles = vec![];
    let result = Arc::new(Mutex::new(0.0));
    let mut current_pos = 0;

    for i in 0..num_threads {
        let chunk_size = chunk_size + if i < chunk_size_overflow { 1 } else { 0 };
        let left = &buffer[current_pos..current_pos + chunk_size];
        let right = &buffer[half_size + current_pos..half_size + current_pos + chunk_size];
        let result = Arc::clone(&result);

        handles.push(thread::spawn(move || {
            let mut chunk_result = 0.0;
            for j in 0..chunk_size {
                let x = left[j] as f64;
                let y = right[j] as f64;
                chunk_result += (x.cos() * y.sin()).abs().sqrt();
            }
            let mut total = result.lock().unwrap();
            *total += chunk_result;
        }));

        current_pos += chunk_size;
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("{}", *result.lock().unwrap());

    // Unmap the memory
    unsafe {
        munmap(buffer.as_ptr() as *mut u8, size * std::mem::size_of::<i32>());
    }

    Ok(())
} 