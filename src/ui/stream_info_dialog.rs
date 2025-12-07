use adw::prelude::*;
use gtk::Window;

use crate::audio::stream_info::StreamInfo;

pub fn show(parent: Option<&Window>, info: StreamInfo) {
    let mut properties = vec![
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
        properties.push(("Bit Rate", format!("{} kbps", bit_rate)));
    }
    if let Some(encoder) = info.encoder {
        properties.push(("Encoder", encoder));
    }

    // Create the grid for key/value pairs
    let info_grid = gtk::Grid::new();
    info_grid.set_column_spacing(12);
    info_grid.set_row_spacing(6);
    info_grid.set_margin_top(12);
    info_grid.set_margin_bottom(12);
    info_grid.set_margin_start(12);
    info_grid.set_margin_end(12);

    for (index, ele) in properties.iter().enumerate() {
        let label = gtk::Label::new(Some(&format!("{}:", ele.0)));
        let value = gtk::Label::new(Some(&ele.1));
        label.set_halign(gtk::Align::End);
        value.set_halign(gtk::Align::Start);
        label.add_css_class("heading");
        info_grid.attach(&label, 0, index as i32, 1, 1);
        info_grid.attach(&value, 1, index as i32, 1, 1);
    }

    // Create a header bar with title
    let header_bar = adw::HeaderBar::new();
    header_bar.set_title_widget(Some(&adw::WindowTitle::new("Stream Info", "")));

    // Create a toolbar view to combine header and content
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&info_grid));

    let dialog = adw::Dialog::builder()
        .can_close(true)
        .child(&toolbar_view)
        .build();

    dialog.present(parent);
}
