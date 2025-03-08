use std::path::PathBuf;

use arboard::Clipboard;
use clap::Parser;
use csv::Reader;
use cursive::{theme::{BorderStyle, Palette, Theme}, views::{Dialog, TextView}};
use serde::Deserialize;

#[derive(clap::Parser)]
#[clap(version)]
/// A simple UI to make ED Neutron Star plotting less manual
struct Args {
    /// Input csv data from spansh filename
    csv_file: PathBuf,
}

fn yesno_bool<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;

    match &*s {
        "Yes" => Ok(true),
        "No" => Ok(false),

        s => {
            use serde::de::Error;
            Err(D::Error::unknown_variant(s, &["Yes", "No"]))
        }
    }
}

#[derive(serde_derive::Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct CsvEntry {
    #[serde(rename = "System Name")]
    system_name: String,
    distance: f64,
    #[serde(rename = "Distance Remaining")]
    distance_remaining: f64,
    #[serde(rename = "Fuel Left")]
    fuel_left: f64,
    #[serde(rename = "Fuel Used")]
    fuel_used: f64,
    #[serde(deserialize_with = "yesno_bool")]
    refuel: bool,
    #[serde(rename = "Neutron Star", deserialize_with = "yesno_bool")]
    neutron_star: bool,
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let args = Args::parse();
    let clipboard = Clipboard::new()?;

    let records: Vec<CsvEntry> = Reader::from_path(args.csv_file)?
        .deserialize()
        .collect::<Result<_, _>>()?;

    let (f, l) = match (records.first(), records.last()) {
        (Some(f), Some(l)) => (f, l),
        _ => return Err("No Systems in CSV file".into()),
    };

    println!("\x1b[37mRoute Summary:");
    println!(
        "  \x1b[37mJourney: \x1b[96m{} \x1b[94m==\x1b[91m{:.3}kly\x1b[94m==> \x1b[96m{}",
        f.system_name,
        f.distance_remaining / 1000.,
        l.system_name
    );
    println!("  \x1b[37mTotal Jumps: \x1b[96m{}", records.len() - 1);
    println!(
        "  \x1b[37mNeutron Stars: \x1b[96m{}\x1b[0m",
        records.iter().filter(|system| system.neutron_star).count()
    );
    println!(
        "  \x1b[37mFuel Stops: \x1b[96m{}\x1b[0m",
        records.iter().filter(|system| system.refuel).count()
    );

    let mut ui = cursive::termion();

    ui.set_theme(Theme {
        shadow: false,
        borders: BorderStyle::Outset,
        palette: Palette::terminal_default(),
    });

    ui.add_global_callback('q', |s| s.quit());

    ui.add_layer(Dialog::new().button("Quit", |s| s.quit()).button("hi", |_|{}));

    ui.run();

    Ok(())
}
