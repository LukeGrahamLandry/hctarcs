use std::env;
use std::process::Command;
use ril::prelude::*;

#[test]
fn first_images() -> anyhow::Result<()> {
    env::set_current_dir("..")?;
    println!("{:?}", env::current_dir());

    // TODO: struct for including deny flags
    let my_tests = &["mandelbrot", "stamp_pos", "sanity"][..];
    let vendor_tests = &["linrays"][..];
    let (w, h) = (240usize, 180usize);
    let mut image = Image::new((w * my_tests.len().max(vendor_tests.len())) as u32, (h * 2) as u32, Rgb::black());

    // Don't make cargo re-check that the compiler's compiled each test.
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--package")
        .arg("compiler");

    // TODO: threads? most time is spent waiting on cargo
    // TODO: just log failures in the loop so you can see them all
    // TODO: run fmt if a test fails
    for (y, tests) in [my_tests, vendor_tests].iter().enumerate() {
        for (x, name) in tests.iter().enumerate() {
            let mut cmd = Command::new("./target/debug/compiler");
            cmd.arg("-i")
                .arg(format!("target/{}.sb3", name))
                .arg("-o")
                .arg(format!("out/gen/{}", name))
                .arg("--first-frame-only");
            assert!(cmd.status()?.success());

            // TODO: put name as text on the image. need to get a font.
            if let Ok(img) = Image::open(&format!("out/gen/{name}/frame.png")) {
                image.paste((x * w) as u32, (y * h) as u32, &img.resized(w as u32, h as u32, ResizeAlgorithm::Nearest));
            }
        }
    }

    image.save_inferred("out/all.png")?;
    assert!(Command::new("open").arg("out/all.png").status()?.success());

    Ok(())
}
