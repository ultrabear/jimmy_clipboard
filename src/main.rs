use std::path::PathBuf;

use arboard::Clipboard;
use clap::Parser;
use csv::Reader;
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
    distance: String,
    #[serde(rename = "Distance Remaining")]
    distance_remaining: String,
    #[serde(rename = "Fuel Left")]
    fuel_left: String,
    #[serde(rename = "Fuel Used")]
    fuel_used: String,
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



    Ok(())
}
