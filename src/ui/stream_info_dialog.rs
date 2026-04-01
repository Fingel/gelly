use adw::prelude::*;
use gtk::Window;

use crate::{audio::stream_info::StreamInfo, config};

fn add_to_grid(row: i32, col_offset: i32, prop: &(&str, String), grid: &gtk::Grid) {
    let key = gtk::Label::new(Some(prop.0));
    let value = gtk::Label::new(Some(&prop.1));
    key.set_halign(gtk::Align::End);
    value.set_halign(gtk::Align::Start);
    key.add_css_class("dim-label");
    grid.attach(&key, col_offset, row, 1, 1);
    grid.attach(&value, col_offset + 1, row, 1, 1);
}

fn yes_no(value: Option<bool>) -> String {
    match value {
        Some(true) => "Yes".to_string(),
        Some(false) => "No".to_string(),
        None => "Unknown".to_string(),
    }
}

pub fn show(parent: Option<&Window>, info: StreamInfo) {
    // Local properties
    let mut left_props = vec![
        ("Backend", config::get_backend_type().as_str().to_string()),
        ("Codec", info.codec.unwrap_or("Unknown".to_string())),
        (
            "Container",
            info.container_format.unwrap_or("None".to_string()),
        ),
        (
            "Sample Rate",
            format!("{} Hz", info.sample_rate.unwrap_or(0)),
        ),
        ("Channels", info.channels.unwrap_or(0).to_string()),
    ];
    if let Some(bit_rate) = info.bit_rate {
        left_props.push(("Bit Rate", format!("{} kbps", bit_rate)));
    }
    if let Some(encoder) = info.encoder {
        left_props.push(("Encoder", encoder));
    }

    // Remote properties
    let mut right_props = vec![
        (
            "Original Codec",
            info.original_codec.unwrap_or("Unknown".to_string()),
        ),
        (
            "Original Container",
            info.original_container_format.unwrap_or("None".to_string()),
        ),
        (
            "Original Sample Rate",
            info.original_sample_rate.unwrap_or(0).to_string(),
        ),
        (
            "Original Channels",
            info.original_channels.unwrap_or(0).to_string(),
        ),
        ("Supports Direct Play", yes_no(info.supports_direct_play)),
        (
            "Supports Direct Stream",
            yes_no(info.supports_direct_stream),
        ),
        ("Supports Transcoding", yes_no(info.supports_transcoding)),
    ];
    if let Some(original_bit_rate) = info.original_bit_rate {
        right_props.push((
            "Original Bit Rate",
            format!("{:.1} kbps", original_bit_rate / 1000),
        ));
    }
    if let Some(file_size) = info.file_size {
        right_props.push(("File Size", format!("{:.1} MB", file_size / 1024 / 1024)));
    }

    let num_rows = left_props.len().max(right_props.len()) as i32;

    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(6);
    grid.set_margin_top(12);
    grid.set_margin_bottom(12);
    grid.set_margin_start(12);
    grid.set_margin_end(12);

    let sep = gtk::Separator::new(gtk::Orientation::Vertical);
    sep.set_margin_start(4);
    sep.set_margin_end(4);
    grid.attach(&sep, 2, 0, 1, num_rows);

    for (row, prop) in left_props.iter().enumerate() {
        add_to_grid(row as i32, 0, prop, &grid);
    }
    for (row, prop) in right_props.iter().enumerate() {
        add_to_grid(row as i32, 3, prop, &grid);
    }

    // Create a header bar with title
    let header_bar = adw::HeaderBar::new();
    header_bar.set_title_widget(Some(&adw::WindowTitle::new("Stream Info", "")));

    // Create a toolbar view to combine header and content
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    let whats_in_the_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    whats_in_the_box.append(&grid);
    toolbar_view.set_content(Some(&whats_in_the_box));

    let dialog = adw::Dialog::builder()
        .can_close(true)
        .child(&toolbar_view)
        .build();

    dialog.present(parent);
}
