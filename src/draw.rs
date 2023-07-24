use super::{DataLine, State, OUT_FILE_NAME};

use plotters::{
    coord::{types::RangedCoordf64, Shift},
    prelude::*,
};
use std::ops::Range;

struct SlicedData<'a, Time> {
    charging: Vec<&'a [DataLine<Time>]>,
    discharging: Vec<&'a [DataLine<Time>]>,
}

fn split_slices(data: &[DataLine<chrono::NaiveDateTime>]) -> SlicedData<chrono::NaiveDateTime> {
    let mut charging = Vec::new();
    let mut discharging = Vec::new();

    let mut state = data[0].state;
    let mut start = 0;
    for (idx, cur_state) in data.iter().map(|d| d.state).enumerate().skip(1) {
        if cur_state != state {
            // starting a new section
            // push to the old section (subtraction ok because skip 1)
            let to_push = &data[start..idx - 1];
            match state {
                State::Charging => charging.push(to_push),
                State::Discharging => discharging.push(to_push),
            }
            // update metadata used for iteration
            // might be a way to do this functionally instead of with a loop? not sure
            state = cur_state;
            start = idx;
        }
    }

    // last slice not dealt with in loop
    let to_push = &data[start..];
    match state {
        State::Charging => charging.push(to_push),
        State::Discharging => discharging.push(to_push),
    }

    SlicedData {
        charging,
        discharging,
    }
}

struct ChartOptions<'a, X, Y> {
    name: &'a str,
    x_range: Range<X>,
    y_range: Range<Y>,
    y_style: YStyle,
}

fn setup_root(out_file: &str) -> DrawingArea<SVGBackend, Shift> {
    let background_color = RGBColor(0x1e, 0x1e, 0x2e);
    // set up canvas
    let root = SVGBackend::new(out_file, (1024, 768)).into_drawing_area();
    root.fill(&background_color).unwrap();

    root
}

fn setup_chart<'root_ref, 'out_file_path>(
    root: &'root_ref DrawingArea<SVGBackend<'out_file_path>, Shift>,
    options: ChartOptions<chrono::NaiveDateTime, f64>,
) -> ChartContext<
    'root_ref,
    SVGBackend<'out_file_path>,
    Cartesian2d<RangedDateTime<chrono::NaiveDateTime>, RangedCoordf64>,
> {
    let text_color = RGBAColor(0xcd, 0xd6, 0xf4, 1.);
    let text_style = ("sans-serif", (5).percent_height())
        .with_color(text_color)
        .into_text_style(root);

    let mut chart = ChartBuilder::on(root)
        .caption(options.name, text_style)
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .margin((2).percent())
        .build_cartesian_2d(RangedDateTime::from(options.x_range), options.y_range)
        .unwrap();

    chart
        .configure_mesh()
        .disable_mesh()
        .axis_style(ShapeStyle {
            color: text_color,
            filled: false,
            stroke_width: 1,
        })
        .label_style(
            ("sans-serif", (2.5).percent_height())
                .with_color(text_color)
                .into_text_style(root),
        )
        .x_label_formatter(&|x| x.format("%H:%M").to_string())
        .y_label_formatter(match options.y_style {
            YStyle::Unchanged => &y_default,
            YStyle::Hours => &y_hours,
        })
        .draw()
        .unwrap();

    chart
}

fn y_default(y: &f64) -> String {
    format!("{y}")
}

fn y_hours(y: &f64) -> String {
    let hours = (y / 3600.) as usize;
    let mins = (y / 60.) as usize % 60;
    format!("{hours}:{mins:>02}")
}

pub(crate) enum MaxStyle {
    Max,
    Constant(f64),
}

pub(crate) enum YStyle {
    Unchanged,
    Hours,
}

pub(crate) fn draw_chart(
    data: Vec<DataLine<chrono::NaiveDateTime>>,
    title: &str,
    y_max: MaxStyle,
    y_style: YStyle,
) {
    let options = {
        let first = data.first().unwrap();
        let last = data.last().unwrap();
        ChartOptions {
            name: title,
            x_range: first.date..last.date,
            y_range: 0.0..match y_max {
                MaxStyle::Constant(max) => max,
                MaxStyle::Max => data
                    .iter()
                    .map(|d| d.value)
                    .fold(0., |max: f64, cur| max.max(cur)),
            },
            y_style,
        }
    };

    let root = setup_root(OUT_FILE_NAME);
    let mut chart = setup_chart(&root, options);

    let red = RGBColor(0xf3, 0x8b, 0xa8);
    let green = RGBColor(0xa6, 0xe3, 0xa1);

    let split_data = split_slices(&data);

    for slice in split_data.discharging {
        chart
            .draw_series(LineSeries::new(
                slice.into_iter().map(|d| (d.date, d.value)),
                red,
            ))
            .unwrap();
    }

    for slice in split_data.charging {
        chart
            .draw_series(LineSeries::new(
                slice.into_iter().map(|d| (d.date, d.value)),
                green,
            ))
            .unwrap();
    }
}
