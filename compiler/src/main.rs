fn main() {
    #[cfg(feature = "cli")]
    cli::run().unwrap();

    // Don't bother pulling in dependencies if only want to use the library part.
    #[cfg(not(feature = "cli"))]
    compile_error!("cli disabled")
}

#[cfg(feature = "cli")]
mod cli {
    use std::fs;
    use std::fs::{create_dir_all, File};
    use std::io::{Cursor, Read, read_to_string};
    use std::path::PathBuf;
    use std::process::Command;
    use clap::{Parser, ValueEnum};
    use serde::Deserialize;
    use zip::ZipArchive;
    use compiler::{AssetPackaging, Target};
    use compiler::ast::Project;
    use compiler::backend::rust::{emit_rust, make_cargo_toml};
    use compiler::scratch_schema::parse;

    pub(crate) fn run() -> anyhow::Result<()> {
        let opts = Cli::parse();
        assert_eq!(opts.assets, AssetPackaging::Embed);

        let (project, name) = if opts.input.starts_with("http") {
            assert!(opts.input.contains("scratch.mit.edu/projects") || opts.input.contains("turbowarp.org"));
            let id = opts.input.split("?").nth(0).unwrap().trim_end_matches("/").split("/").last().unwrap().trim();
            let id_num = id.parse::<u64>()?;

            let res = ureq::get(&format!("https://api.scratch.mit.edu/projects/{id}"))
                .set("User-Agent", &opts.user_agent)
                .call()?;
            let info: ScratchApiResponse = res.into_json()?;
            // TODO: put author credit in cargo toml
            println!("Fetching project {}: {}", info.id, info.title);
            assert_eq!(id_num, info.id);
            let raw = ureq::get(&format!("https://projects.scratch.mit.edu/{id}?token={}", info.project_token))
                .set("User-Agent", &opts.user_agent)
                .call()?.into_string()?;

            let project = parse(raw.as_str())?;

            (project, format!("s{id}"))
        } else if opts.input.ends_with(".sb3") {
            let bytes = fs::read(PathBuf::from(&opts.input))?;
            let mut zip = ZipArchive::new(Cursor::new(bytes))?;
            let proj = zip.by_name("project.json")?;
            let raw = read_to_string(proj)?;
            let mut path = opts.outdir.clone();
            create_dir_all(&path)?;
            path.push("project.json");
            fs::write(path, raw.as_str())?;
            let project = parse(raw.as_str())?;
            let name = PathBuf::from(opts.input).file_name().unwrap().to_string_lossy().replace(['.'], "_");

            // TODO: if Assets::fetch, do a dry run to make sure scratch has the referenced things (it might be a local sb3 never shared)
            // TODO: Assets::fetch option to change base url
            // TODO be smarter about this for web.
            project.targets
                .iter()
                .flat_map(|t| t.costumes.iter())
                .for_each(|c| {
                    let mut file = zip.by_name(&c.md5ext).unwrap();
                    let mut path = opts.outdir.clone();
                    path.push("src");
                    path.push("assets");
                    fs::create_dir_all(&path).unwrap();
                    path.push(c.md5ext.clone());
                    let mut buf = vec![];
                    file.read_to_end(&mut buf).unwrap();
                    fs::write(path, buf).unwrap();
                });

            (project, name)
        } else {
            panic!("Unsupported input. Expected url or local .sb3 file path.");
        };


        let project: Project = project.into();
        let result = emit_rust(&project, opts.render, opts.assets);

        let mut path = opts.outdir.clone();
        path.push("src");
        create_dir_all(&path)?;
        path.push("main.rs");
        fs::write(path, &result)?;

        let cargotoml = match opts.cargotoml {
            None => make_cargo_toml(opts.render, opts.assets).replace("scratch_out", &name),
            Some(path) => fs::read_to_string(path)?,
        };
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

        if opts.deny_poly {
            assert_eq!(result.matches("Poly").count(), 0, "Failed to infer all types.");
        }

        Ok(())
    }

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct Cli {
        // TODO:  or project.json local. or number id of scratch project
        /// Local path to a .sb3 file OR url to scratch/turbowarp project.
        #[arg(short, long)]
        input: String,

        /// Where to put generated src/main.rs and and Cargo.toml
        #[arg(short, long)]
        outdir: PathBuf,

        #[arg(long, default_value="macroquad")]
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

        /// Assert that no polymorphic types were used in the program. Used for testing. Should probably be moved to a script.
        #[arg(long)]
        deny_poly: bool,

        /// What to use as the User-Agent http header when the compiler calls the scratch api.
        #[arg(long, default_value = "github/LukeGrahamLandry/hctarcs")]
        user_agent: String,

        #[arg(long, default_value="embed")]
        assets: AssetPackaging,

        #[arg(long)]
        wasm: bool
    }

    #[derive(Deserialize)]
    struct ScratchApiResponse {
        id: u64,
        title: String,
        project_token: String,
        // author: ScratchUser
    }

    // TODO
    // #[derive(Deserialize)]
    // struct ScratchUser {
    //     username: String,
    // }
}

