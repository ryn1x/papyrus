use crate::pfh::*;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::{error, fmt, fs};

pub fn compile<P, F>(
    compile_dir: P,
    linking_config: &linking::LinkingConfiguration,
    stderr_line_cb: F,
) -> Result<PathBuf, CompilationError>
where
    P: AsRef<Path>,
    F: Fn(&str),
{
    let compile_dir = compile_dir.as_ref();
    let lib_file = compile_dir.join("target/debug/");
    let lib_file = if cfg!(windows) {
        lib_file.join(format!("{}.dll", LIBRARY_NAME))
    } else {
        lib_file.join(format!("lib{}.so", LIBRARY_NAME))
    };

    let mut _s_tmp = String::new();
    let mut args = vec!["rustc", "--", "-Awarnings"];
    if let Some(crate_name) = linking_config.crate_name {
        args.push("--extern");
        _s_tmp = format!("{0}=lib{0}.rlib", crate_name);
        args.push(&_s_tmp);
    }

    let mut child = Command::new("cargo")
        .current_dir(compile_dir)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| CompilationError::NoBuildCommand)?;

    let stderr = {
        let rdr = BufReader::new(child.stderr.as_mut().expect("stderr should be piped"));
        let mut s = String::new();
        for line in rdr.lines() {
            let line = line.unwrap();
            stderr_line_cb(&line);
            s.push_str(&line);
            s.push('\n');
        }
        s
    };

    match child.wait() {
        Ok(ex) => {
            if ex.success() {
                Ok(lib_file)
            } else {
                Err(CompilationError::CompileError(stderr))
            }
        }
        Err(e) => Err(CompilationError::IOError(e)),
    }
}

/// Error type for compilation.
#[derive(Debug)]
pub enum CompilationError {
    /// Failed to initialise `cargo build`. Usually because `cargo` is not in your `PATH` or Rust is not installed.
    NoBuildCommand,
    /// A compiling error occured, with the contents of the stderr.
    CompileError(String),
    /// Generic IO errors.
    IOError(io::Error),
}

impl error::Error for CompilationError {}

impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompilationError::NoBuildCommand => {
                write!(f, "cargo build command failed to start, is rust installed?")
            }
            CompilationError::CompileError(e) => write!(f, "{}", e),
            CompilationError::IOError(e) => write!(f, "io error occurred: {}", e),
        }
    }
}

#[test]
fn compilation_error_fmt_test() {
    let e = CompilationError::NoBuildCommand;
    assert_eq!(
        &e.to_string(),
        "cargo build command failed to start, is rust installed?"
    );
    let e = CompilationError::CompileError("compile err".to_string());
    assert_eq!(&e.to_string(), "compile err");
    let ioe = io::Error::new(io::ErrorKind::Other, "test");
    let e = CompilationError::IOError(ioe);
    assert_eq!(&e.to_string(), "io error occurred: test");
}
