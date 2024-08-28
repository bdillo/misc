use libc::{
    c_uint, c_void, close, dup2, execve, fork, open, pipe, printf, read, wait, waitpid, write,
    O_CREAT, O_TRUNC, O_WRONLY, STDIN_FILENO, STDOUT_FILENO, WEXITSTATUS, WIFEXITED,
};
use std::{
    cmp::Ordering,
    error::Error,
    ffi::CString,
    fs::{read_to_string, remove_file},
    path::Path,
    process,
    ptr::null,
    thread::sleep,
    time::Duration,
};

type HwResult = Result<(), Box<dyn Error>>;

fn logit(pid: u32, msg: &str) {
    eprintln!("[{pid}] {msg}");
}

fn fork_panic() {
    panic!("fork failed!");
}

fn q1() -> HwResult {
    let mut x = 12;
    unsafe {
        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                logit(current_pid, &format!("x is {x}"));
                x += 1;
                logit(current_pid, &format!("x is now {x}"));
            }
            Ordering::Greater => {
                logit(current_pid, &format!("x is {x}"));
                x -= 1;
                logit(current_pid, &format!("x is now {x}"));
            }
        }
    }

    Ok(())
}

fn q2() -> HwResult {
    let filename = "/tmp/ostep-hw".to_owned();
    let file_path = Path::new(&filename);

    let c_path = CString::new(filename.clone())?;
    let c_path_p = c_path.as_ptr();

    let buf_len = 5;
    let mut buf: [u8; 5] = *b"abcde";
    let buf_p = buf.as_mut_ptr() as *mut c_void;

    unsafe {
        let fd = open(c_path_p, O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_uint);

        let pid = fork();
        let current_pid = process::id();

        if pid < 0 {
            fork_panic();
        }

        logit(current_pid, &format!("writing to fd {fd}"));
        write(fd, buf_p, buf_len);

        if pid == 0 {
            process::exit(0);
        } else {
            let mut status = 0;
            wait(&mut status);
            logit(current_pid, &format!("closing fd {fd}"));
            close(fd);
        }
    }

    eprintln!("file contents: {}", read_to_string(file_path)?);

    // cleanup
    remove_file(file_path)?;

    Ok(())
}

fn q3() -> HwResult {
    unsafe {
        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => logit(current_pid, "hello"),
            Ordering::Greater => {
                sleep(Duration::from_millis(1));
                logit(current_pid, "goodbye");
            }
        }
    }

    Ok(())
}

fn q4() -> HwResult {
    // was creating pointers with as_ptr() here, but actually this means the CStrings don't live long enough!
    let executable = CString::new("/bin/ls")?;
    let arg1 = CString::new("-lah")?;
    let arg2 = CString::new("/bin")?;

    unsafe {
        let pid = fork();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                let args_p = [executable.as_ptr(), arg1.as_ptr(), arg2.as_ptr(), null()].as_ptr();
                let env_p = [null()].as_ptr();

                let ret = execve(executable.as_ptr(), args_p, env_p);
                // shouldn't print
                eprintln!("execve ret: {ret}");
            }
            Ordering::Greater => (),
        }
    }

    Ok(())
}

fn q5() -> HwResult {
    unsafe {
        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                logit(current_pid, "hello from child!");
                process::exit(0);
            }
            Ordering::Greater => {
                let mut status = 0;
                wait(&mut status);

                if WIFEXITED(status) {
                    logit(
                        current_pid,
                        &format!("done waiting, wait status: {}", WEXITSTATUS(status)),
                    );
                } else {
                    logit(current_pid, &format!("error, proc returned: {status}"));
                    panic!("error waiting on child");
                }
            }
        }
    }

    Ok(())
}

fn q6() -> HwResult {
    unsafe {
        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                logit(current_pid, "hello from child!");
                process::exit(0);
            }
            Ordering::Greater => {
                let mut status = 0;
                waitpid(pid, &mut status, 0);

                if WIFEXITED(status) {
                    logit(
                        current_pid,
                        &format!("done waiting, wait status: {}", WEXITSTATUS(status)),
                    );
                } else {
                    logit(current_pid, &format!("error, proc returned: {status}"));
                    panic!("error waiting on child");
                }
            }
        }
    }

    Ok(())
}

fn q7() -> HwResult {
    let format_str = CString::new("hello %s!")?;
    let world = CString::new("world")?;

    unsafe {
        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                close(STDOUT_FILENO);
                printf(format_str.as_ptr(), world.as_ptr());
                // funny we can still use eprintln!
                logit(current_pid, "hello from child!");
                process::exit(0);
            }
            Ordering::Greater => (),
        }
    }

    Ok(())
}

fn q8() -> HwResult {
    unsafe {
        let mut pipe_fds = [0; 2];
        if pipe(pipe_fds.as_mut_ptr()) == -1 {
            panic!("pipe failed!");
        };

        let read_pipe = pipe_fds[0];
        let write_pipe = pipe_fds[1];

        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                logit(current_pid, "hello from child #1!");
                // take stdout into pipe
                dup2(write_pipe, STDOUT_FILENO);

                // don't need these after dup
                close(read_pipe);
                close(write_pipe);

                let buf = CString::new("hello from the pipe!\n")?;
                let buf_len = buf.as_bytes().len();

                write(STDOUT_FILENO, buf.as_ptr() as *const c_void, buf_len);

                process::exit(0);
            }
            Ordering::Greater => (),
        }

        let pid = fork();
        let current_pid = process::id();

        match pid.cmp(&0) {
            Ordering::Less => fork_panic(),
            Ordering::Equal => {
                logit(current_pid, "hello from child #2!");
                // read from pipe as stdin
                dup2(read_pipe, STDIN_FILENO);

                // don't need these after dup
                close(read_pipe);
                close(write_pipe);

                let mut buf = [0u8; 1024];
                read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, buf.len());
                write(STDOUT_FILENO, buf.as_ptr() as *const c_void, buf.len());

                process::exit(0);
            }
            Ordering::Greater => (),
        }

        let mut status = 0;
        wait(&mut status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q1() -> HwResult {
        q1()
    }

    #[test]
    fn test_q2() -> HwResult {
        q2()
    }

    #[test]
    fn test_q3() -> HwResult {
        q3()
    }

    #[test]
    fn test_q4() -> HwResult {
        q4()
    }

    #[test]
    fn test_q5() -> HwResult {
        q5()
    }

    #[test]
    fn test_q6() -> HwResult {
        q6()
    }

    #[test]
    fn test_q7() -> HwResult {
        q7()
    }

    #[test]
    fn test_q8() -> HwResult {
        q8()
    }
}
