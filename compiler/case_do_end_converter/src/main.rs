/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use case_do_end_converter::rewrite_case_do_end;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

enum Mode {
    Stdout,
    Check,
    Out(PathBuf),
}

#[derive(Default)]
struct Summary {
    files_scanned: usize,
    files_written: usize,
    files_changed: usize,
}

fn usage(program: &str) {
    eprintln!("Usage: {} --out <output-dir> <input-file-or-dir>", program);
    eprintln!("       {} --check <input-file-or-dir>", program);
    eprintln!("       {} <input-file>", program);
    eprintln!();
    eprintln!("Without a mode flag, rewritten source is printed to stdout.");
}

fn parse_args(args: &[String]) -> Result<(Mode, PathBuf), String> {
    match args {
        [_program, input] => Ok((Mode::Stdout, PathBuf::from(input))),
        [_program, mode, input] if mode == "--check" => Ok((Mode::Check, PathBuf::from(input))),
        [_program, mode, output_dir, input] if mode == "--out" => {
            Ok((Mode::Out(PathBuf::from(output_dir)), PathBuf::from(input)))
        }
        [program, ..] => {
            usage(program);
            Err("invalid arguments".to_string())
        }
        [] => Err("missing program name".to_string()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (mode, input_path) = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("Error: {}", message);
            std::process::exit(2);
        }
    };

    let result = match mode {
        Mode::Stdout => print_rewritten_file(&input_path),
        Mode::Check => check_path(&input_path),
        Mode::Out(output_root) => write_output_tree(&input_path, &output_root),
    };

    if let Err(message) = result {
        eprintln!("Error: {}", message);
        std::process::exit(1);
    }
}

fn print_rewritten_file(input_path: &Path) -> Result<(), String> {
    if !input_path.is_file() {
        return Err("stdout mode requires a single input file".to_string());
    }

    let source = read_source(input_path)?;
    let rewritten = rewrite_source(input_path, &source)?;
    print!("{}", rewritten);
    Ok(())
}

fn check_path(input_path: &Path) -> Result<(), String> {
    let mut summary = Summary::default();
    let files = collect_input_files(input_path)?;

    for file in files {
        let source = read_source(&file)?;
        let rewritten = rewrite_source(&file, &source)?;
        summary.files_scanned += 1;

        if rewritten != source {
            summary.files_changed += 1;
            eprintln!("{} needs case do/end conversion", file.display());
        }
    }

    print_check_summary(&summary);

    if summary.files_changed > 0 {
        return Err(format!("{} file(s) need conversion", summary.files_changed));
    }

    Ok(())
}

fn write_output_tree(input_path: &Path, output_root: &Path) -> Result<(), String> {
    validate_output_root(input_path, output_root)?;

    let input_root = if input_path.is_dir() {
        input_path
    } else {
        input_path
            .parent()
            .ok_or_else(|| format!("Cannot find parent for {}", input_path.display()))?
    };

    let files = collect_input_files(input_path)?;
    let mut summary = Summary::default();

    for input_file in files {
        let source = read_source(&input_file)?;
        let rewritten = rewrite_source(&input_file, &source)?;
        let relative_path = input_file
            .strip_prefix(input_root)
            .map_err(|err| format!("Cannot compute relative path for {}: {}", input_file.display(), err))?;
        let output_file = output_root.join(relative_path);

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Cannot create {}: {}", parent.display(), err))?;
        }

        fs::write(&output_file, rewritten.as_bytes())
            .map_err(|err| format!("Cannot write {}: {}", output_file.display(), err))?;

        summary.files_scanned += 1;
        summary.files_written += 1;
        if rewritten != source {
            summary.files_changed += 1;
        }
    }

    print_out_summary(&summary, output_root);
    Ok(())
}

fn collect_input_files(input_path: &Path) -> Result<Vec<PathBuf>, String> {
    if input_path.is_file() {
        return Ok(vec![input_path.to_path_buf()]);
    }

    if !input_path.is_dir() {
        return Err(format!("{} is not a file or directory", input_path.display()));
    }

    let mut files = Vec::new();
    collect_silica_files(input_path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_silica_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|err| format!("Cannot read directory {}: {}", path.display(), err))?;

    for entry in entries {
        let entry = entry.map_err(|err| format!("Cannot read directory entry in {}: {}", path.display(), err))?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_silica_files(&entry_path, files)?;
        } else if entry_path.is_file() && is_silica_source(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn is_silica_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "silica")
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|err| format!("Cannot read {}: {}", path.display(), err))
}

fn rewrite_source(path: &Path, source: &str) -> Result<String, String> {
    rewrite_case_do_end(source, &path.display().to_string())
        .map_err(|err| format!("Cannot rewrite {}: {}", path.display(), err))
}

fn validate_output_root(input_path: &Path, output_root: &Path) -> Result<(), String> {
    if output_root == input_path {
        return Err("output directory must not be the same as input path".to_string());
    }

    if input_path.is_dir() && output_root.starts_with(input_path) {
        return Err("output directory must not be inside the input directory".to_string());
    }

    Ok(())
}

fn print_check_summary(summary: &Summary) {
    eprintln!("files scanned: {}", summary.files_scanned);
    eprintln!("files changed: {}", summary.files_changed);
}

fn print_out_summary(summary: &Summary, output_root: &Path) {
    eprintln!("files scanned: {}", summary.files_scanned);
    eprintln!("files written: {}", summary.files_written);
    eprintln!("files changed: {}", summary.files_changed);
    eprintln!("output: {}", output_root.display());
}
