#![allow(non_upper_case_globals)]
use std::env;
use std::path::PathBuf;
use std::process::Command;
use ril::prelude::*;
use compiler::cli::{Cli, run};

// TODO: move this to a bin?
// TODO: add a bin that builds web demos

// TODO: recompile the src files as needed
const my_tests: &[T] = &[
    T { name: "mandelbrot", deny_async: true, deny_poly: true },
    T { name: "stamp_pos", deny_async: true, deny_poly: true },
    T { name: "sanity", deny_async: true, deny_poly: false }
    ];

// TODO: recompile/fetch the src files as needed
// TODO: add tres once async works (rn it would just hang waiting for input)
const vendor_tests: &[T] = &[
    T { name: "linrays", deny_async: true, deny_poly: true },
    ];

#[test]
fn first_images() -> anyhow::Result<()> {
    env::set_current_dir("..")?;
    println!("{:?}", env::current_dir());

    let (w, h) = (240usize, 180usize);
    let mut image = Image::new((w * my_tests.len().max(vendor_tests.len())) as u32, (h * 2) as u32, Rgb::black());

    // TODO: threads? most time is spent waiting on cargo
    // TODO: just log failures in the loop so you can see them all
    // TODO: run fmt if a test fails
    for (y, tests) in [my_tests, vendor_tests].iter().enumerate() {
        for (x, test) in tests.iter().enumerate() {
            run(test.opts())?;

            // TODO: put name as text on the image. need to get a font.
            if let Ok(img) = Image::open(&format!("out/gen/{}/frame.png", test.name)) {
                image.paste((x * w) as u32, (y * h) as u32, &img.resized(w as u32, h as u32, ResizeAlgorithm::Nearest));
            }
        }
    }

    image.save_inferred("out/all.png")?;
    assert!(Command::new("open").arg("out/all.png").status()?.success());

    Ok(())
}

struct T {
    name: &'static str,
    deny_async: bool,
    deny_poly: bool
}

impl T {
    fn opts(&self) -> Cli {
        let &T { name, deny_async, deny_poly } = self;
        Cli {
            input: format!("target/{}.sb3", name),
            outdir: PathBuf::from(format!("out/gen/{}", name)),
            first_frame_only: true,
            deny_async,
            deny_poly,
            ..Default::default()
        }
    }
}
