use std::env;
use std::fs::File;
use std::io::{self, Error, ErrorKind};
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
    if args.len() < 3 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Usage: {} <filepath> <size>", args[0]),
        ));
    }

    let size: usize = args[2]
        .parse()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Size must be a positive integer"))?;
    
    if size == 0 {
        return Err(Error::new(ErrorKind::InvalidInput, "Size must be a positive integer"));
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

    let result: f64 = (0..half_size)
        .map(|i| {
            let x = buffer[i] as f64;
            let y = buffer[half_size + i] as f64;
            (x.cos() * y.sin()).abs().sqrt()
        })
        .sum();

    println!("{}", result);

    // Unmap the memory
    unsafe {
        munmap(buffer.as_ptr() as *mut u8, size * std::mem::size_of::<i32>());
    }

    Ok(())
} 