use std::{error::Error, ffi::CString};

use libc::{close, open, O_RDONLY};

extern crate test;

fn add_two(a: i32) -> i32 {
    a + 2
}

fn syscall() -> Result<(), Box<dyn Error>> {
    let c_path = CString::new("/dev/urandom")?;
    unsafe {
        let fd = open(c_path.as_ptr(), O_RDONLY);
        close(fd);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    // just for reference in the benchmarks
    #[bench]
    fn bench_add_two(b: &mut Bencher) {
        b.iter(|| add_two(2));
    }

    #[bench]
    fn bench_syscall(b: &mut Bencher) {
        b.iter(|| syscall());
    }
}

/*

NOTE: skipping the context switch bench as I don't feel like learning about macos core affinity rn, maybe will come
back and write one for linux

bench results:

...
test chapter_6::tests::bench_add_two ... bench:           0.25 ns/iter (+/- 0.00)
test chapter_6::tests::bench_syscall ... bench:       4,675.59 ns/iter (+/- 45.38)

test result: ok. 0 passed; 0 failed; 9 ignored; 2 measured; 0 filtered out; finished in 2.41s

*/
