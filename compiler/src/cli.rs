use std::fs;
use std::fs::create_dir_all;
use std::io::{Cursor, Read, read_to_string};
use std::path::PathBuf;
use std::process::{Command};
use clap::Parser;
use serde::Deserialize;
use zip::ZipArchive;
use crate::{AssetPackaging, Target};
use crate::ast::Project;
use crate::backend::rust::{emit_rust};
use crate::scratch_schema::parse;

pub fn run(opts: Cli) -> anyhow::Result<()> {
    for name in &["main_rs", "sprite_body"] {
        assert!(opts.get_template_path(name).is_none(), "Overriding template {name} is not implemented.");
    }

    if let Some(_id) = opts.log_default_template {
        todo!("I actually can't do this cause of how my macros work. ");
        // exit(0);
    }

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
        let name = PathBuf::from(&opts.input).file_name().unwrap().to_string_lossy().replace(['.'], "_");

        // TODO: if Assets::fetch, do a dry run to make sure scratch has the referenced things (it might be a local sb3 never shared)
        // TODO: Assets::fetch option to change base url
        // TODO be smarter about this for web.
        project.targets
            .iter()
            .flat_map(|t| t.costumes.iter())
            .for_each(|c| {
                // TOOD: instead of unwrap, if its not there somehow try fetching that hash from scratch api? more useful once i support raw project.json without bundled
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

    if opts.deny_async {
        assert!(!project.any_async, "Made async calls but --deny-async");
    }
    let result = emit_rust(&project, opts.render, opts.assets);

    let mut path = opts.outdir.clone();
    path.push("src");
    create_dir_all(&path)?;
    path.push("main.rs");
    fs::write(path, &result)?;

    let mut path = opts.outdir.clone();
    path.push(".gitignore");
    fs::write(path, "target\nproject.json\n.DS_Store\n")?;

    let cargotoml = template!(opts, "data/cargo_toml", name=&name, backend=opts.render.code_name());
    let mut path = opts.outdir.clone();
    path.push("Cargo.toml");
    fs::write(path, cargotoml)?;

    // TODO: output this in the web demo too
    let mut path = opts.outdir.clone();
    path.push("rust-toolchain.toml");
    fs::write(path, "[toolchain]\nchannel = \"nightly\"")?;

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

    let warning = template!(opts, "data/macroquad_warning", );
    if opts.web {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("--release");
        cmd.arg("--target");
        cmd.arg("wasm32-unknown-unknown");
        assert!(cmd.current_dir(&opts.outdir).status()?.success());
        let mut wasm = opts.outdir.clone();
        loop {  // Look upwards encase its a workspace
            wasm.push("target");
            if wasm.exists() {
                break;
            } else {
                assert!(wasm.pop() && wasm.pop(), "Failed to find target directory. ")
            }
        }
        let mut web_folder = wasm.clone();
        web_folder.push("web");
        wasm.push("wasm32-unknown-unknown");
        wasm.push("release");
        wasm.push(format!("{name}.wasm"));
        assert!(wasm.exists(), "File Not Found {wasm:?}");

        create_dir_all(&web_folder)?;
        let mut old_wasm = web_folder.clone();
        old_wasm.push(format!("{name}.wasm"));
        fs::copy(&wasm, old_wasm)?;
        let mut cmd = Command::new("wasm-bindgen");
        cmd.arg(wasm).arg("--target").arg("web").arg("--out-dir").arg(&web_folder);
        let mut bindjs = web_folder.clone();
        bindjs.push(format!("{name}.js"));
        assert!(cmd.status()?.success());

        // HACK HACK HACK
        assert_eq!(opts.render, Target::Macroquad);
        let mut genjs = fs::read_to_string(&bindjs)?;
        for (before, after) in PATCHES {  // TODO: iffy replacement but none are in expressions
            assert!(genjs.contains(before), "idk this version of wasm-bindgen?");  // TODO: can i use find's index so dont have to search twice?
            genjs = genjs.replace(before, &format!("{BREAD_CRUMBS}\n/* (REMOVED) {before} */\n/*ADDED:*/{after}\n/*END PATCH*/"));
        }
        fs::write(&bindjs, genjs)?;

        let mut index = web_folder.clone();
        index.push("index.html");
        // TODO: allow user provided html template and inject the hack
        let data = template!(opts, "data/macroquad_index", mq_js_bundle_url="./mq_js_bundle.js", hint=BREAD_CRUMBS, warning_comment=warning, name=&name);
        fs::write(&index, data)?;

        // TODO: ugly cause what happens when you're using a different version. does cargo fetch the src?
        let macroquad = ureq::get("https://raw.githubusercontent.com/not-fl3/macroquad/master/js/mq_js_bundle.js")
            .set("User-Agent", &opts.user_agent)
            .call()?.into_string()?;
        let mut mq_js_bundle = web_folder.clone();
        mq_js_bundle.push("mq_js_bundle.js");
        fs::write(&mq_js_bundle, &macroquad)?;
    }

    if opts.render == Target::Macroquad {
        println!("{warning}");
    }

    if opts.run || opts.debug || opts.first_frame_only{
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if !opts.debug {
            cmd.arg("--release");
        }
        if opts.first_frame_only {
            cmd.arg("--").arg("--first-frame-only");
        }
        assert!(cmd.current_dir(&opts.outdir).status()?.success());
    }

    if opts.deny_poly {
        assert_eq!(result.matches("Poly").count(), 0, "Failed to infer all types but --deny-poly");
    }

    Ok(())
}

// TODO: allow overriding these
// Note: kinda relying on these all being statements but easily fixable
const PATCHES: &[(&str, &str)] = &[
    ("import * as __wbg_star0 from 'env';", ""),
    ("let wasm;", "let wasm; export const set_wasm = (w) => wasm = w;"),
    ("imports['env'] = __wbg_star0;", "return imports.wbg;"),
    ("const imports = __wbg_get_imports();", "return __wbg_get_imports();"),
];

// TODO: store templates elsewhere and include them? they could be fmt literals and use with format! and named params

// TODO: cli flag to turn this off
/// Just in case someone comes across the deranged hack I borrowed, give them a unique string to google for
const BREAD_CRUMBS: &str = "/* Hint: HACK-PATCH-github-LukeGrahamLandry-Hctarcs 2024-01-04 */";

// Foot gun: Default::default() is different than clap's cli default.
#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    // TODO:  or project.json local. or number id of scratch project
    /// Local path to a .sb3 file OR url to scratch/turbowarp project.
    #[arg(short, long)]
    pub input: String,

    /// Where to put generated src/main.rs and and Cargo.toml
    #[arg(short, long)]
    pub outdir: PathBuf,

    /// Which graphics library to use.
    ///
    /// Not all backends are guaranteed to support all features but the default is a superset of the rest.
    #[arg(long, default_value="macroquad")]
    pub render: Target,

    /// How will the executable find image/sound files. Trade off between binary size and reliability.
    #[arg(long, default_value="embed")]
    pub assets: AssetPackaging,

    /// What to use as the User-Agent http header when the compiler calls the scratch api.
    #[arg(long, default_value = "github/LukeGrahamLandry/hctarcs")]
    pub user_agent: String,

    /// Build to wasm, run wasm-bindgen, and generate an html shim.
    ///
    /// In theory this is just a convince over `cargo build --target wasm32-... && wasm-bindgen ...` but some backends need special hacks to make that work.
    #[arg(long)]
    pub web: bool,

    /// Run for one frame, save a screenshot, and then exit (same as building normally then passing --first-frame-only to the exe)
    #[arg(long)]
    pub first_frame_only: bool,

    // TODO: UNUSED thus far
    /// Override output templates used in the compiler. Format: "id=filepath"
    /// TODO: parameter replacement not implemented
    #[arg(long)]
    pub template: Option<Vec<String>>,

    // TODO: sub command? so you can do this without specifying input/output that are just ignored
    /// TODO: not implemented.
    /// Instead of compiling, log the default content of a template, and exit. Useful for seeing which parameters you can override.
    #[arg(long)]
    pub log_default_template: Option<String>,

    /// Assert that no polymorphic types were used in the program. Used for testing. Should probably be moved to a script.
    #[arg(long)]
    pub deny_poly: bool,

    /// Assert that no async calls were used in the program. Used for testing. Should probably be moved to a script.
    #[arg(long)]
    pub deny_async: bool,

    /// Use cargo fmt on the output.
    #[arg(long)]
    pub fmt: bool,

    /// Use cargo check on the output.
    #[arg(long)]
    pub check: bool,

    /// Use cargo run --release on the output.
    #[arg(long)]
    pub run: bool,

    /// Use cargo run on the output.
    #[arg(long)]
    pub debug: bool,


    // TODO: --sc1 (and walk up file tree to find assets folder to put cwd)
}

impl Cli {
    fn get_template_path(&self, name: &str) -> Option<&str> {
        // TODO: HACK until i figure out abs paths. also should use pathbuf function?
        let name = name.split("/").last().unwrap();  // lookup key is a magic id, not a path
        self.template.iter().map(|templates| {  // TODO: untested
            templates.iter().find_map(|arg| {
                let mut parts = arg.split('=');
                let key = parts.next().expect("Override key.").trim();
                let value = parts.next().expect("Override value.").trim();
                assert!(parts.next().is_none(), "Expect one '=' in template override");

                if key == name {
                    println!("[DEBUG] template {name} overridden to {value}");
                    Some(value)
                } else {
                    None
                }
            })
        }).flatten().next()
    }
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
