use super::*;
use failure::ResultExt;
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::{error, fmt};

/// The resulting compiled executable.
pub struct Exe {
	path: String,
}

/// A current operating child process.
pub struct Process {
	child: Child,
}

/// A current compiling process.
pub struct CompilingProcess {
	exe: Exe,
	process: Process,
}

/// Error type for compilation.
#[derive(Debug)]
pub enum InitialisingError {
	/// Failed to initialise `cargo build`. Usually because `cargo` is not in your `PATH` or Rust is not installed.
	NoBuildCommand,
	/// Generic IO errors.
	IOError(String),
}
/// Error type for compilation.
#[derive(Debug)]
pub struct CompileError;

impl error::Error for InitialisingError {}

impl fmt::Display for InitialisingError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			InitialisingError::NoBuildCommand => {
				write!(f, "cargo build command failed to start, is rust installed?")
			}
			InitialisingError::IOError(e) => write!(f, "io error occurred. {}", e),
		}
	}
}

impl error::Error for CompileError {}

impl fmt::Display for CompileError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "compilation failed")
	}
}

impl Exe {
	/// Compile a `SourceFile` in the given directory.
	pub fn compile<P: AsRef<Path>>(
		src: &SourceFile,
		compile_dir: P,
	) -> Result<CompilingProcess, InitialisingError> {
		build_compile_dir(src, &compile_dir)?;

		let mut exe = format!(
			"{}/target/debug/{}",
			compile_dir.as_ref().to_string_lossy(),
			src.file_name
		);
		if cfg!(windows) {
			exe.push_str(".exe");
		}
		match Command::new("cargo")
			.current_dir(compile_dir)
			.arg("rustc")
			.args(&["--", "-Awarnings"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
		{
			Ok(c) => Ok(CompilingProcess {
				exe: Exe { path: exe },
				process: Process { child: c },
			}),
			Err(_) => Err(InitialisingError::NoBuildCommand),
		}
	}

	/// Run the `Exe`.
	pub fn run<P: AsRef<Path>>(&self, working_dir: P) -> Process {
		Process {
			child: Command::new(&self.path)
				.current_dir(working_dir)
				.env("RUST_BACKTRACE", "0")
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()
				.expect(&format!(
					"failed to start the executable {}, which is unlikely.",
					self.path
				)),
		}
	}
}

impl Process {
	/// Wait for the process to finish.
	pub fn wait(mut self) -> ExitStatus {
		self.child
			.wait()
			.expect("failed waiting for process to finish")
	}

	/// The `stderr` handle.
	pub fn stderr(&mut self) -> &mut ChildStderr {
		self.child.stderr.as_mut().expect("stderr should be piped")
	}

	/// The `stdout` handle.
	pub fn stdout(&mut self) -> &mut ChildStdout {
		self.child.stdout.as_mut().expect("stderr should be piped")
	}
}

impl CompilingProcess {
	/// Wait for the process to finish. Is successful, a `Exe` pointer will be returned, which can be run.
	pub fn wait(self) -> Result<Exe, CompileError> {
		if self.process.wait().success() {
			Ok(self.exe)
		} else {
			Err(CompileError)
		}
	}

	/// The `stderr` handle.
	pub fn stderr(&mut self) -> &mut ChildStderr {
		self.process.stderr()
	}
}

/// Constructs the compile directory with the given main source file contents.
/// Expects `SourceFileType::Rs` to define a `main()` function.
/// `SourceFileType::Rscript` will encase code in a `main()` function.
fn build_compile_dir<P: AsRef<Path>>(
	source: &SourceFile,
	compile_dir: &P,
) -> Result<(), InitialisingError> {
	let compile_dir = compile_dir.as_ref();
	let mut main_file = create_file_and_dir(&compile_dir.join("src/main.rs"))
		.map_err(|e| InitialisingError::IOError(e.to_string()))?;
	let mut cargo_file = create_file_and_dir(&compile_dir.join("Cargo.toml"))
		.map_err(|e| InitialisingError::IOError(e.to_string()))?;
	let cargo = cargotoml_contents(source);
	let content = main_contents(source);
	main_file
		.write_all(content.as_bytes())
		.context("failed writing contents of main.rs".to_string())
		.map_err(|e| InitialisingError::IOError(e.to_string()))?;
	cargo_file
		.write_all(cargo.as_bytes())
		.context("failed writing contents of Cargo.toml".to_string())
		.map_err(|e| InitialisingError::IOError(e.to_string()))?;
	Ok(())
}

fn cargotoml_contents(source: &SourceFile) -> String {
	format!(
		r#"[package]
name = "{pkg_name}"
version = "0.1.0"

[dependencies]
{crates}
"#,
		pkg_name = source.file_name,
		crates = source
			.crates
			.iter()
			.map(|c| format!(r#"{} = "*""#, c.cargo_name))
			.collect::<Vec<_>>()
			.join("\n")
	)
}

fn main_contents(source: &SourceFile) -> String {
	format!(
		r#"
{crates}

{src}
"#,
		crates = source
			.crates
			.iter()
			.map(|c| c.src_line.clone())
			.collect::<Vec<_>>()
			.join("\n"),
		src = match source.file_type {
			SourceFileType::Rs => source.src.clone(),
			SourceFileType::Rscript => format!(
				r#"fn main() {{
	{}
}}"#,
				source.src
			),
		}
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	const TEST_CONTENTS: &str = "fn main() { println!(\"Hello, world!\"); }";

	#[test]
	fn test_build_compile_dir() {
		let source = SourceFile {
			src: TEST_CONTENTS.to_string(),
			file_type: SourceFileType::Rs,
			file_name: "test-name".to_string(),
			crates: Vec::new(),
		};

		build_compile_dir(&source, &"tests/compile-dir/test-dir").unwrap();
		assert!(Path::new("tests/compile-dir/test-dir/src/main.rs").exists());
		assert!(Path::new("tests/compile-dir/test-dir/Cargo.toml").exists());

		fs::remove_dir_all("tests/compile-dir/test-dir").unwrap();
	}

	#[test]
	fn test_run_success() {
		use std::env;
		let dir = "tests/compile-dir/test-run";
		let source = SourceFile {
			src: TEST_CONTENTS.to_string(),
			file_type: SourceFileType::Rs,
			file_name: "test-name".to_string(),
			crates: Vec::new(),
		};
		assert!(Exe::compile(&source, &dir)
			.unwrap()
			.wait()
			.unwrap()
			.run(&env::current_dir().unwrap())
			.wait()
			.success());

		fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn fail_compile() {
		let dir = "tests/compile-dir/test-run";

		let source = SourceFile {
			src: "fn main() { let a = 1 }".to_string(),
			file_type: SourceFileType::Rs,
			file_name: "test-name".to_string(),
			crates: Vec::new(),
		};

		match Exe::compile(&source, &dir).unwrap().wait() {
			Err(_) => (),
			_ => panic!("expecting compilation error"),
		}

		fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn fail_runtime() {
		use std::env;
		let dir = "tests/compile-dir/test-run";

		let source = SourceFile {
			src: r#"fn main() { panic!("runtime error!"); }"#.to_string(),
			file_type: SourceFileType::Rs,
			file_name: "test-name".to_string(),
			crates: Vec::new(),
		};
		let r = Exe::compile(&source, &dir)
			.unwrap()
			.wait()
			.unwrap()
			.run(&env::current_dir().unwrap())
			.wait();
		assert!(!r.success());

		fs::remove_dir_all(dir).unwrap();
	}
}