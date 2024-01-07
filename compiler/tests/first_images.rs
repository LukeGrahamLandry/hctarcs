#![allow(non_upper_case_globals)]
use std::env;
use std::path::PathBuf;
use std::process::Command;
use ril::prelude::*;
use compiler::cli::{Cli, run};

// TODO: move this to a bin?
// TODO: add a bin that builds web demos
// TODO: runtime arg for stdin/stdout say/input so can test tres. test api for rust giving it a list of triggers (generic over Msg).
// TODO: runtime flag for save frame on exit for testing tres

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
    T { name: "tres", deny_async: false, deny_poly: false },
    ];

const temp_tests: &[T] = &[
    T { name: "async", deny_async: false, deny_poly: false },
];


#[test]
fn first_images() -> anyhow::Result<()> {
    env::set_current_dir("..")?;
    println!("{:?}", env::current_dir());

    // TODO: fix size so dont include the debug ui
    let (view_w, view_h) = (480 * 2, 360 * 2);
    let (w, h) = (240usize, 180usize);
    let mut image = Image::new((w * my_tests.len().max(vendor_tests.len())) as u32, (h * 3) as u32, Rgb::black());

    // TODO: threads? most time is spent waiting on cargo
    // TODO: just log failures in the loop so you can see them all
    // TODO: run fmt if a test fails
    for (y, tests) in [my_tests, vendor_tests, temp_tests].iter().enumerate() {
        for (x, test) in tests.iter().enumerate() {
            run(test.opts())?;

            // TODO: put name as text on the image. need to get a font.
            if let Ok(img) = Image::open(&format!("out/gen/{}/frame.png", test.name)) {
                image.paste((x * w) as u32, (y * h) as u32, &img.cropped(0, 0, view_w, view_h).resized(w as u32, h as u32, ResizeAlgorithm::Nearest));
            }
        }
    }

    image.save_inferred("target/all.png")?;
    assert!(Command::new("open").arg("target/all.png").status()?.success());

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
