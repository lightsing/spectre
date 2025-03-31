use clap::Parser;
use console::{Emoji, style};
use spectre::*;
use std::{
    fs::{File, read_to_string},
    io::Write,
    path::PathBuf,
    process::exit,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(default_value = "spectre.toml")]
    builder: PathBuf,
    #[clap(default_value = "witness.json")]
    out: PathBuf,

    #[clap(long, help = "Create a new builder file")]
    new: bool,
}

static ERROR: Emoji<'_, '_> = Emoji("❌  ", ":-( ");
static TEMPLATE: &str = include_str!("../../../examples/full.toml");

#[tokio::main]
async fn main() {
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

    let builder = read_to_string(&args.builder)
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
    let witnesses = spectre
        .trace()
        .await
        .inspect_err(|e| {
            eprintln!(
                "{ERROR}{}",
                style(format!("error when dump witness: {}", e)).bold()
            );
            exit(1);
        })
        .unwrap();

    eprintln!(
        "{}witness dumped in {:?}",
        Emoji("✨  ", ":-) "),
        now.elapsed()
    );

    let filename = args.out.file_name().unwrap().to_string_lossy();

    let create_file = |path| {
        File::create(path)
            .inspect_err(|e| {
                eprintln!(
                    "{ERROR}{}",
                    style(format!("error creating file: {}", e)).bold()
                );
                exit(1);
            })
            .unwrap()
    };

    let write_witness = |file: File, witnesses: &BlockWitness| {
        serde_json::to_writer_pretty(file, witnesses)
            .inspect_err(|e| {
                eprintln!(
                    "{ERROR}{}",
                    style(format!("error writing trace: {}", e)).bold()
                );
                exit(1);
            })
            .unwrap();
    };

    if witnesses.len() > 1 {
        eprintln!("{}more than one block used", Emoji("😮️  ", ":O "));

        for (idx, witness) in witnesses.iter().enumerate() {
            let path = args.out.with_file_name(format!("{filename}-{idx}"));
            let file = create_file(path);
            write_witness(file, witness);
        }
    } else {
        let file = create_file(args.out);
        write_witness(file, &witnesses[0]);
    }
}
