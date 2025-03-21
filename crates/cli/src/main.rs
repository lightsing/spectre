use clap::Parser;
use console::{style, Emoji};
use spectre::*;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(default_value = "spectre.toml")]
    builder: PathBuf,
    #[clap(default_value = "trace.json")]
    out: PathBuf,

    #[clap(long, help = "Create a new builder file")]
    new: bool,
}

static ERROR: Emoji<'_, '_> = Emoji("❌  ", ":-( ");
static TEMPLATE: &str = include_str!("../../example.toml");

fn main() {
    let args = Args::parse();
    if args.new {
        let mut file = File::create("spectre.toml")
            .inspect_err(|e| {
                eprintln!(
                    "{ERROR}{}",
                    style(format!("error creating file: {}", e)).bold()
                );
                exit(1);
            })
            .unwrap();
        file.write_all(TEMPLATE.as_bytes())
            .inspect_err(|e| {
                eprintln!(
                    "{ERROR}{}",
                    style(format!("error writing file: {}", e)).bold()
                );
                exit(1);
            })
            .unwrap();
        return;
    }

    let builder = read_to_string(args.builder)
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error opening file: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();

    let builder: SpectreBuilder = toml::from_str(&builder)
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error parsing builder: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();

    let spectre = builder
        .build()
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error building spectre: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();

    eprintln!("{spectre}");

    let now = std::time::Instant::now();
    let trace = spectre
        .trace()
        .inspect_err(|e| {
            eprintln!("{ERROR}{}", style(format!("error tracing: {}", e)).bold());
            exit(1);
        })
        .unwrap();

    eprintln!("{}traced in {:?}", Emoji("✨  ", ":-) "), now.elapsed());

    let out = File::create(args.out)
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error creating file: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();

    serde_json::to_writer_pretty(out, &trace)
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error writing trace: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();
}
