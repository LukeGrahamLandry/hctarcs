use std::ffi::OsStr;
use std::fs;
use std::fs::create_dir_all;
use std::io::{Cursor, read_to_string};
use std::path::PathBuf;
use std::process::Command;
use clap::{Parser, ValueEnum};
use zip::ZipArchive;
use compiler::ast::Project;
use compiler::backend::rust::{emit_rust, make_cargo_toml};
use compiler::scratch_schema::parse;

// TODO: put this in makefile somehow since it doesnt really belong here
// assert_eq!(linrays.matches("Poly").count(), 0, "You broke type inference.");

fn main() -> anyhow::Result<()> {
    let opts = Cli::parse();

    let ext = opts.input.extension();
    let is_sb3 = match ext {
        Some(s) => s == "sb3" || opts.force,
        None => opts.force,
    };
    if !is_sb3 {
        panic!("Unrecognised file extension {} (expected sb3). Use --force to continue anyway.", ext.map(OsStr::to_str).flatten().unwrap_or("<NONE>"))
    }
    let bytes = fs::read(&opts.input)?;
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    let proj = zip.by_name("project.json")?;
    let raw = read_to_string(proj)?;
    let project = parse(raw.as_str())?;
    let project: Project = project.into();
    let result = emit_rust(&project);

    let mut path = opts.outdir.clone();
    path.push("src");
    create_dir_all(&path)?;
    path.push("main.rs");
    fs::write(path, &result)?;

    let name = opts.input.file_name().unwrap().to_string_lossy().replace(['.'], "_");
    let cargotoml = make_cargo_toml(opts.render.code_name()).replace("scratch_out", &name);
    let mut path = opts.outdir.clone();
    path.push("Cargo.toml");
    fs::write(path, cargotoml)?;

    // println!("[INFO] {} Polymorphic Type Annotations", result.matches(": Poly").count());
    // println!("[INFO] {} Polymorphic Constructor Calls", result.matches("Poly::from").count());

    // Note: Starting a new process hangs forever when running in RustRover's profiler???
    if opts.fmt {
        // Using output() even when i only want status makes it not print it.
        assert!(Command::new("cargo").arg("fmt").current_dir(&opts.outdir).output()?.status.success());
    }
    if opts.check {
        let output = Command::new("cargo").arg("check").current_dir(&opts.outdir).output()?;
        if !output.status.success() {
            println!("{}", String::from_utf8(output.stdout)?);
            println!("{}", String::from_utf8(output.stderr)?);
            panic!("ICE: cargo check failed.");
        }
    }

    if opts.run || opts.debug {
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if !opts.debug {
            cmd.arg("--release");
        }
        assert!(cmd.current_dir(&opts.outdir).status()?.success());
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to a .sb3 file or project.json
    #[arg(short, long)]
    input: PathBuf,

    /// Where to put generated src/main.rs and and Cargo.toml
    #[arg(short, long)]
    outdir: PathBuf,

    #[arg(long, default_value="notan")]
    render: Target,

    /// Use cargo fmt on the output.
    #[arg(long)]
    fmt: bool,

    /// Use cargo check on the output.
    #[arg(long)]
    check: bool,

    /// Use cargo run --release on the output.
    #[arg(long)]
    run: bool,

    /// Use cargo run on the output.
    #[arg(long)]
    debug: bool,

    /// Path to your own file to use instead of the default generated Cargo.toml
    #[arg(long)]
    cargotoml: Option<PathBuf>,

    /// Ignore some classes of error: unrecognised file extension.
    #[arg(long, default_value_t=false)]
    force: bool
}

#[derive(Default, Copy, Clone, Debug, ValueEnum)]
pub enum Target {
    #[default]
    Notan,
    Softbuffer,
}

impl Target {
    fn code_name(&self) -> &str {
        match self {
            Target::Notan => "notan",
            Target::Softbuffer => "softbuffer",
        }
    }
}
