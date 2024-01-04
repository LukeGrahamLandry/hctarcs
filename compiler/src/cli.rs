use std::fs;
use std::fs::create_dir_all;
use std::io::{Cursor, Read, read_to_string};
use std::path::PathBuf;
use std::process::Command;
use clap::Parser;
use serde::Deserialize;
use zip::ZipArchive;
use crate::{AssetPackaging, Target};
use crate::ast::Project;
use crate::backend::rust::{emit_rust, make_cargo_toml};
use crate::scratch_schema::parse;

pub fn run(opts: Cli) -> anyhow::Result<()> {
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

    let cargotoml = match opts.cargotoml {
        None => make_cargo_toml(opts.render, opts.assets).replace("scratch_out", &name),
        Some(path) => fs::read_to_string(path)?,
    };
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
        fs::write(&index, MACROQUAD_INDEX_HTML(&name, "./mq_js_bundle.js"))?;

        // TODO: ugly cause what happens when you're using a different version. does cargo fetch the src?
        let macroquad = ureq::get("https://raw.githubusercontent.com/not-fl3/macroquad/master/js/mq_js_bundle.js")
            .set("User-Agent", &opts.user_agent)
            .call()?.into_string()?;
        let mut mq_js_bundle = web_folder.clone();
        mq_js_bundle.push("mq_js_bundle.js");
        fs::write(&mq_js_bundle, &macroquad)?;
    }

    if opts.render == Target::Macroquad {
        println!("{MACROQUAD_WASM_WARNING}");
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

// Note: kinda relying on these all being statements but easily fixable
const PATCHES: &[(&str, &str)] = &[
    ("import * as __wbg_star0 from 'env';", ""),
    ("let wasm;", "let wasm; export const set_wasm = (w) => wasm = w;"),
    ("imports['env'] = __wbg_star0;", "return imports.wbg;"),
    ("const imports = __wbg_get_imports();", "return __wbg_get_imports();"),
];

// TODO: cli flag to turn this off
/// Just in case someone comes across the deranged hack I borrowed, give them a unique string to google for
const BREAD_CRUMBS: &str = "/* Hint: HACK-PATCH-github-LukeGrahamLandry-Hctarcs 2024-01-04 */";

// TODO: instead of printing at this comptime, have a cargo feature that we pass on the command line when compiling for wasm and [cfg(all(not(feature="allow_sketchy_wasm"), arch="wasm32"))] compiler_error!(this message)
// TODO: that way i dont have to always print but can give hint to avoid confusion and if they know what they're doing they can jsut provide the feature themselves when building
pub const MACROQUAD_WASM_WARNING: &str = r#"WARNING: Macroquad doesn't play nice with wasm-bindgen.
The Hctarcs compiler provides a hacky fix it, but it means the web target cannot be built as a normal cargo+wasm_bindgen project anymore.
You need to run Hctarcs with --web every time. (SAD: cargo build script can't run after compiling).

More Info:
- https://github.com/not-fl3/macroquad/issues/212
- https://github.com/not-fl3/miniquad/wiki/JavaScript-interop
- https://gist.github.com/profan/f38c9a2f3d8c410ad76b493f76977afe
"#;

fn MACROQUAD_INDEX_HTML(name: &str, mq_js_bundle_url: &str) -> String {
    format!(r#"
<html><head><style>
    body {{ text-align: center }}
    canvas {{ overflow: hidden; outline: none; display: inline-block; }}
</style></head><body>
<canvas id="glcanvas" tabindex='1' width="480" height="360"></canvas>
<script src="{mq_js_bundle_url}"></script>
<script type="module">
    {BREAD_CRUMBS}
    /*{MACROQUAD_WASM_WARNING}*/
    // This also requires the compiler to hack in postprocessing step to the wasm_bg_js
    import init, {{ set_wasm }} from "./{name}.js";
    let wbg = await init();
    miniquad_add_plugin({{
        register_plugin: (importObject) => {{
            // In fact, unless someone else is messing with it, importObject.wbg should === undefined
            assert(Object.keys((importObject.wbg || {{}})).length <= Object.keys(wbg || {{}}).length, "What mq_js_bundle exposes to wasm-bindgen should be a subset of what wasm-bindgen asks for.");
            importObject.wbg = wbg;
            // When wasm-bindgen gets its hands on the import object, it will move things from [wbg] to [env] where rust can access them.
            console.log("the 'No __wbg_... function in gl.js' warnings are probably fine :)"); // TODO make them go away
        }},
        on_init: () => set_wasm(wasm_exports),
        version: "0.2.0",
        name: "wbg",
    }});
    load("{name}_bg.wasm");
</script></body></html>
"#, )
}

/// Foot gun: Default::default() is different than clap's cli default.
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

    #[arg(long, default_value="macroquad")]
    pub render: Target,

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

    /// Path to your own file to use instead of the default generated Cargo.toml
    #[arg(long)]
    pub cargotoml: Option<PathBuf>,

    /// Assert that no polymorphic types were used in the program. Used for testing. Should probably be moved to a script.
    #[arg(long)]
    pub deny_poly: bool,

    /// Assert that no async calls were used in the program. Used for testing. Should probably be moved to a script.
    #[arg(long)]
    pub deny_async: bool,

    /// What to use as the User-Agent http header when the compiler calls the scratch api.
    #[arg(long, default_value = "github/LukeGrahamLandry/hctarcs")]
    pub user_agent: String,

    #[arg(long, default_value="embed")]
    pub assets: AssetPackaging,

    #[arg(long)]
    pub web: bool,

    /// Run for one frame, save a screenshot, and then exit (same as building normally then passing --first-frame-only to the exe)
    #[arg(long)]
    pub first_frame_only: bool,

    // TODO: --sc1 (and walk up file tree to find assets folder to put cwd)
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