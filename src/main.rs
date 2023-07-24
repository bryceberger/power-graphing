#![feature(type_changing_struct_update)]

mod draw;

use chrono::TimeZone;
use serde::Deserialize;
use std::path::Path;

const OUT_FILE_NAME: &str = "/tmp/out.svg";

type Timestamp = i64;
#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum State {
    Charging,
    Discharging,
}

#[derive(Deserialize, Debug)]
struct DataLine<Time> {
    date: Time,
    value: f64,
    state: State,
}

fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<DataLine<Timestamp>>, csv::Error> {
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_path(path)?;

    Ok(reader.into_deserialize().filter_map(Result::ok).collect())
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("{0}")]
    Csv(#[from] csv::Error),
    #[error("All data filtered out")]
    FilteredOut,
}

fn filter_after_time(
    data: impl IntoIterator<Item = DataLine<Timestamp>>,
    remove_before: Timestamp,
) -> impl Iterator<Item = DataLine<Timestamp>> {
    data.into_iter().skip_while(move |d| d.date < remove_before)
}

fn timestamp_to_local_datetime(date: Timestamp) -> chrono::NaiveDateTime {
    chrono::offset::Local
        .from_utc_datetime(&chrono::NaiveDateTime::from_timestamp_opt(date, 0).unwrap_or_default())
        .naive_local()
}

enum FileType {
    Rate,
    Charge,
    Empty,
    Full,
}

impl AsRef<Path> for FileType {
    fn as_ref(&self) -> &Path {
        use FileType::*;
        &match self {
            Rate => Path::new("/var/lib/upower/history-rate-ASUS_Battery-76.dat"),
            Charge => Path::new("/var/lib/upower/history-charge-ASUS_Battery-76.dat"),
            Empty => Path::new("/var/lib/upower/history-time-empty-ASUS_Battery-76.dat"),
            Full => Path::new("/var/lib/upower/history-time-full-ASUS_Battery-76.dat"),
        }
    }
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();

    let (chart_title, max_style, y_style, default_hours, data) =
        match args.get(1).map(String::as_str).unwrap_or_default() {
            "rate" => (
                "Rate",
                draw::MaxStyle::Max,
                draw::YStyle::Unchanged,
                2.,
                read_file(FileType::Rate)?,
            ),
            "empty" => (
                "Time to Empty",
                draw::MaxStyle::Max,
                draw::YStyle::Hours,
                6.,
                read_file(FileType::Empty)?,
            ),
            "full" => (
                "Time to Full",
                draw::MaxStyle::Max,
                draw::YStyle::Hours,
                2.,
                read_file(FileType::Full)?,
            ),
            "charge" | _ => (
                "Charge",
                draw::MaxStyle::Constant(100.),
                draw::YStyle::Unchanged,
                6.,
                read_file(FileType::Charge)?,
            ),
        };

    let hours_to_show = args
        .get(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| default_hours);

    let some_time_ago =
        chrono::Utc::now() - chrono::Duration::seconds((hours_to_show * 3600.) as i64);
    let timestamp = some_time_ago.timestamp();

    let data: Vec<_> = filter_after_time(data, timestamp)
        .map(|d| DataLine {
            date: timestamp_to_local_datetime(d.date),
            ..d
        })
        .collect();

    if data.len() == 0 {
        return Err(Error::FilteredOut);
    }

    draw::draw_chart(data, chart_title, max_style, y_style);

    Ok(())
}
