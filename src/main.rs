use core::fmt;
use std::path::PathBuf;

use arboard::Clipboard;
use clap::Parser;
use csv::Reader;
use cursive::{
    theme::{BorderStyle, Palette, Theme},
    view::{Finder, IntoBoxedView, Nameable, SizeConstraint},
    views::{LinearLayout, ListChild, ListView, Panel, ResizedView, ScrollView, TextView},
};
use serde::Deserialize;
use std::process::Command;

#[derive(clap::Parser)]
#[clap(version)]
/// A simple UI to make ED Neutron Star plotting less manual
struct Args {
    /// Input csv data from spansh filename
    csv_file: PathBuf,

    /// Enable focus stealing to ED once a next system copy request has been made
    #[arg(short, long)]
    focus_steal: bool,
}

fn focus_elite() {
    _ = Command::new("wmctrl")
        .args(["-a", "Elite - Dangerous (CLIENT)"])
        .spawn()
        .and_then(|mut v| v.wait());
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

impl CsvEntry {
    fn star(&self) -> StarClass {
        let Self {
            refuel,
            neutron_star,
            ..
        } = self;

        match (refuel, neutron_star) {
            (true, _) => StarClass::Refuel,
            (_, true) => StarClass::Neutron,
            _ => StarClass::Plain,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum StarClass {
    Neutron,
    Refuel,
    Plain,
}

struct TrueColor {
    rgb: [u8; 3],
    background: bool,
}

impl TrueColor {
    fn bg(r: u8, g: u8, b: u8) -> Self {
        Self {
            rgb: [r, g, b],
            background: true,
        }
    }
}

impl fmt::Display for TrueColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let target = if self.background { "48" } else { "38" };

        let [r, g, b] = self.rgb;

        write!(f, "\x1b[{target};2;{r};{g};{b}m")
    }
}

impl fmt::Display for CsvEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            system_name,
            distance,
            distance_remaining,
            fuel_left,
            fuel_used,
            refuel,
            neutron_star,
        } = self;

        match self.star() {
            StarClass::Refuel => write!(f, "{}", TrueColor::bg(32, 16, 0))?,
            StarClass::Neutron => write!(f, "{}", TrueColor::bg(0, 0, 64))?,
            StarClass::Plain => (),
        }

        writeln!(
            f,
            "\x1b[96m{system_name}\x1b[37m: jump:\x1b[96m{distance:0.1}\x1b[37mly"
        )?;

        write!(
            f,
            "remain:\x1b[96m{distance_remaining:0.1}\x1b[37mly tank:\x1b[96m{fuel_used:.1}\x1b[37mT/\x1b[96m{fuel_left:.1}\x1b[37mT "
        )?;

        match self.star() {
            StarClass::Refuel => write!(f, "\x1b[97mRefuel")?,
            StarClass::Neutron => write!(f, "\x1b[97mNeutron")?,
            StarClass::Plain => (),
        }

        Ok(())
    }
}

fn summary(records: &[CsvEntry]) -> Result<String, String> {
    let (f, l) = match (records.first(), records.last()) {
        (Some(f), Some(l)) => (f, l),
        _ => return Err("No Systems in CSV file".into()),
    };

    let l1 = "\x1b[37mRoute Summary:".to_string();
    let l2 = format!(
        "  \x1b[37mTrip: \x1b[96m{} \x1b[94m=\x1b[91m{:.3}kly\x1b[94m=> \x1b[96m{}",
        f.system_name,
        f.distance_remaining / 1000.,
        l.system_name
    );
    let l3 = format!("  \x1b[37mTotal Jumps: \x1b[96m{}", records.len() - 1);
    let l4 = format!(
        "  \x1b[37mNeutron Stars: \x1b[96m{}\x1b[0m",
        records.iter().filter(|system| system.neutron_star).count()
    );
    let l5 = format!(
        "  \x1b[37mFuel Stops: \x1b[96m{}\x1b[0m",
        records.iter().filter(|system| system.refuel).count()
    );

    Ok([l1, l2, l3, l4, l5].join("\n"))
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let args = Args::parse();
    let mut clipboard = Clipboard::new()?;
    let focus_steal = args.focus_steal;

    let records: Vec<CsvEntry> = Reader::from_path(args.csv_file)?
        .deserialize()
        .collect::<Result<_, _>>()?;

    let spanned = summary(&records)?;

    let mut ui = cursive::termion();

    ui.set_theme(Theme {
        shadow: false,
        borders: BorderStyle::Outset,
        palette: Palette::terminal_default(),
    });

    let records_ref = records.clone();

    ui.add_global_callback('q', |s| s.quit());
    ui.add_global_callback(' ', move |s| {
        let mut view = s
            .screen_mut()
            .find_name::<ListView>("progress")
            .expect("Must Exist");

        let ListChild::Row(prev_id, _) = view.get_row(0) else {
            unreachable!()
        };

        let prev_id: usize = prev_id.parse().expect("only valid indexes of records vec"); 

        view.remove_child(0);
        view.remove_child(0);

        if view.is_empty() {
            s.quit();
        } else {
            let ListChild::Row(id, _) = view.get_row(0) else {
                unreachable!()
            };

            let id = id
                .parse::<usize>()
                .expect("only valid indexes of records vec");

            let sys_name = &records_ref[id].system_name;

            _ = clipboard.set_text(sys_name.clone());

            let mut clipview = s
                .screen_mut()
                .find_name::<TextView>("clip_info")
                .expect("must exist");

            let steal = if focus_steal {
                "\nSetting focus back to Elite Dangerous"
            } else {
                ""
            };

            let last_system = &records_ref[prev_id];
            let next_jump = &records_ref[id];

            let fmt = format!("Copied: '\x1b[96m{sys_name}\x1b[0m' to clipboard{steal}\n\n\x1b[37mCurrent System:\n{last_system}\x1b[0m\n\n\x1b[37mNext Jump:\n{next_jump}\n");

            clipview.set_content(cursive::utils::markup::ansi::parse(fmt));

            if focus_steal {
                focus_elite();
            }
        }
    });

    let mut list = ListView::new();

    list.set_children(
        records
            .iter()
            .enumerate()
            .flat_map(|(idx, r)| {
                [
                    ListChild::Row(
                        idx.to_string(),
                        TextView::new(cursive::utils::markup::ansi::parse(r.to_string()))
                            .into_boxed_view(),
                    ),
                    ListChild::Delimiter,
                ]
            })
            .collect(),
    );

    ui.add_layer(
        LinearLayout::horizontal()
            .child(ScrollView::new(list.with_name("progress")))
            .child(
                LinearLayout::vertical()
                    .child(Panel::new(TextView::new(
                        cursive::utils::markup::ansi::parse(spanned),
                    )))
                    .child(Panel::new(ResizedView::new(
                        SizeConstraint::Free,
                        SizeConstraint::AtLeast(10),
                        TextView::new("Welcome to JimmyClipboard").with_name("clip_info"),
                    ))),
            ),
    );

    ui.run();

    Ok(())
}
