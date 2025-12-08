use adw::prelude::*;
use gtk::Window;

use crate::audio::stream_info::StreamInfo;

fn add_to_grid(index: usize, prop: &(&str, String), grid: &gtk::Grid) {
    let label = gtk::Label::new(Some(&format!("{}:", prop.0)));
    let value = gtk::Label::new(Some(&prop.1));
    label.set_halign(gtk::Align::End);
    value.set_halign(gtk::Align::Start);
    label.add_css_class("heading");
    grid.attach(&label, 0, index as i32, 1, 1);
    grid.attach(&value, 1, index as i32, 1, 1);
}

fn yes_no(value: Option<bool>) -> String {
    match value {
        Some(true) => "Yes".to_string(),
        Some(false) => "No".to_string(),
        None => "Unknown".to_string(),
    }
}

pub fn show(parent: Option<&Window>, info: StreamInfo) {
    // Gstreamer properties
    let mut gst_props = vec![
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
        gst_props.push(("Bit Rate", format!("{} kbps", bit_rate)));
    }
    if let Some(encoder) = info.encoder {
        gst_props.push(("Encoder", encoder));
    }
    // Jellyfin properties
    let mut jelly_props = vec![
        (
            "Original Container",
            info.original_container_format.unwrap_or("None".to_string()),
        ),
        (
            "Original Codec",
            info.original_codec.unwrap_or("Unknown".to_string()),
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
        jelly_props.push((
            "Original Bit Rate",
            format!("{:.1} kbps", original_bit_rate / 1000),
        ));
    }
    if let Some(file_size) = info.file_size {
        jelly_props.push(("File Size", format!("{:.1} MB", file_size / 1024 / 1024)));
    }

    let gst_grid = gtk::Grid::new();
    gst_grid.set_column_spacing(12);
    gst_grid.set_row_spacing(6);
    gst_grid.set_margin_top(2);
    gst_grid.set_margin_bottom(2);
    gst_grid.set_margin_start(12);
    gst_grid.set_margin_end(12);

    for (index, ele) in gst_props.iter().enumerate() {
        add_to_grid(index, ele, &gst_grid);
    }

    let jelly_grid = gtk::Grid::new();
    jelly_grid.set_column_spacing(12);
    jelly_grid.set_row_spacing(6);
    jelly_grid.set_margin_top(2);
    jelly_grid.set_margin_bottom(2);
    jelly_grid.set_margin_start(12);
    jelly_grid.set_margin_end(12);

    for (index, ele) in jelly_props.iter().enumerate() {
        add_to_grid(index, ele, &jelly_grid);
    }

    // Create a header bar with title
    let header_bar = adw::HeaderBar::new();
    header_bar.set_title_widget(Some(&adw::WindowTitle::new("Stream Info", "")));

    // Create a toolbar view to combine header and content
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    let whats_in_the_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    whats_in_the_box.append(&gtk::Label::new(Some("Local")));
    whats_in_the_box.append(&gst_grid);
    whats_in_the_box.append(&gtk::Label::new(Some("Jellyfin")));
    whats_in_the_box.append(&jelly_grid);
    toolbar_view.set_content(Some(&whats_in_the_box));

    let dialog = adw::Dialog::builder()
        .can_close(true)
        .child(&toolbar_view)
        .build();

    dialog.present(parent);
}
