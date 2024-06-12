#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

use std::{fs, io::Write};

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use srtb_integration::{
    ChromaIntegrator, Integrator, RawSrtbFile, SpeedsIntegrator, SpinDifficulty,
};

#[cfg(target_arch = "wasm32")]
fn main() {
    unimplemented!("no cli for wasm");
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Please select the integration mode");
    println!("1. Speed Triggers (Dynamic Track Speed)");
    println!("2. Chroma Triggers (Speen Chroma 2)");
    print!("> ");
    std::io::stdout().flush().expect("failed to flush stdout");

    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .expect("failed to read from stdin");

    let opt: usize = buf.trim().parse().expect("invalid integer");
    let integrator: Box<dyn Integrator> = match opt {
        1 => Box::new(SpeedsIntegrator),
        2 => Box::new(ChromaIntegrator),
        _ => panic!("invalid option"),
    };

    println!("Please select the chart");
    let file = FileDialog::new()
        .add_filter("Spin Rhythm Track Bundle", &["srtb"])
        .pick_file()
        .unwrap();
    println!("Selected: {}", file.display());
    let mut chart = RawSrtbFile::open(&file).unwrap();

    println!("Please select a difficulty");
    for (i, diff) in SpinDifficulty::ALL.iter().enumerate() {
        println!("{}. {}", i + 1, diff);
    }
    print!("> ");
    std::io::stdout().flush().expect("failed to flush stdout");

    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .expect("failed to read from stdin");

    let opt: usize = buf.trim().parse().expect("invalid integer");
    let diff = *SpinDifficulty::ALL
        .get(opt - 1)
        .expect("invalid difficulty selected");

    println!("Please select an action");
    println!("1. Integrate");
    println!("2. Extract");
    println!("3. Remove");
    println!("4. Exit");
    print!("> ");
    let mut buf = String::new();
    std::io::stdout().flush().expect("failed to flush stdout");
    std::io::stdin()
        .read_line(&mut buf)
        .expect("failed to read from stdin");
    let action: i32 = buf.trim_end().parse().expect("invalid integer");

    match action {
        1 => {
            let ext = integrator.file_extension();
            println!("Please select a {} file to integrate", ext);
            let extra_file = FileDialog::new()
                .add_filter(format!("{} file", ext), &[&ext])
                .pick_file()
                .unwrap();
            println!("Selected {}", extra_file.display());
            let data = fs::read_to_string(extra_file).unwrap();
            integrator.integrate(&mut chart, &data, diff).unwrap();
            println!("Integration complete! Please select a saving location");
            let save_location = FileDialog::new()
                .add_filter("Spin Rhythm Track Bundle", &["srtb"])
                .save_file()
                .unwrap();
            chart.save(&save_location).unwrap();
            println!("Saved to {}", save_location.display());
        }
        2 => {
            let res = integrator.extract(&chart, diff).unwrap();
            println!("Extraction complete! Please select a saving location");
            let ext = integrator.file_extension();
            let save_location = FileDialog::new()
                .add_filter(format!("{} file", ext), &[ext])
                .save_file()
                .unwrap();
            fs::write(&save_location, res).unwrap();
            println!("Saved to {}", save_location.display());
        }
        3 => {
            integrator.remove(&mut chart, diff).unwrap();
            println!("Removal complete! Please select a saving location");
            let save_location = FileDialog::new()
                .add_filter("Spin Rhythm Track Bundle", &["srtb"])
                .save_file()
                .unwrap();
            chart.save(&save_location).unwrap();
            println!("Saved to {}", save_location.display());
        }
        4 => {}
        _ => unreachable!(),
    }
}
