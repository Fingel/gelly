use adw::prelude::*;
use gtk::Window;

use crate::i18n::tr;
use crate::{audio::stream_info::StreamInfo, config};

fn yes_no(value: Option<bool>) -> String {
    match value {
        Some(true) => tr("Yes"),
        Some(false) => tr("No"),
        None => tr("Unknown"),
    }
}

fn create_property_row(key: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(key);
    row.set_subtitle(value);
    row.set_subtitle_selectable(true);
    row.set_css_classes(&["property"]);
    row
}

fn create_listbox(properties: &[(String, String)]) -> gtk::ListBox {
    let list_box = gtk::ListBox::new();
    list_box.set_css_classes(&["boxed-list"]);
    list_box.set_selection_mode(gtk::SelectionMode::None);
    for (key, value) in properties.iter() {
        let row = create_property_row(key, value);
        list_box.append(&row);
    }
    list_box
}

pub fn show(parent: Option<&Window>, info: StreamInfo) {
    let mut local_props = vec![
        (
            tr("Backend"),
            config::get_backend_type().as_str().to_string(),
        ),
        (tr("Codec"), info.codec.unwrap_or_else(|| tr("Unknown"))),
        (
            tr("Container"),
            info.container_format.unwrap_or_else(|| tr("None")),
        ),
        (
            tr("Sample Rate"),
            format!("{} Hz", info.sample_rate.unwrap_or(0)),
        ),
        (tr("Channels"), info.channels.unwrap_or(0).to_string()),
    ];
    if let Some(bit_rate) = info.bit_rate {
        local_props.push((tr("Bit Rate"), format!("{} kbps", bit_rate)));
    }
    if let Some(encoder) = info.encoder {
        local_props.push((tr("Encoder"), encoder));
    }

    let mut remote_props = vec![
        (
            tr("Original Codec"),
            info.original_codec.unwrap_or_else(|| tr("Unknown")),
        ),
        (
            tr("Original Container"),
            info.original_container_format.unwrap_or_else(|| tr("None")),
        ),
        (
            tr("Original Sample Rate"),
            info.original_sample_rate.unwrap_or(0).to_string(),
        ),
        (
            tr("Original Channels"),
            info.original_channels.unwrap_or(0).to_string(),
        ),
        (
            tr("Supports Direct Play"),
            yes_no(info.supports_direct_play),
        ),
        (
            tr("Supports Direct Stream"),
            yes_no(info.supports_direct_stream),
        ),
        (
            tr("Supports Transcoding"),
            yes_no(info.supports_transcoding),
        ),
    ];
    if let Some(original_bit_rate) = info.original_bit_rate {
        remote_props.push((
            tr("Original Bit Rate"),
            format!("{:.1} kbps", original_bit_rate / 1000),
        ));
    }
    if let Some(file_size) = info.file_size {
        remote_props.push((
            tr("File Size"),
            format!("{:.1} MB", file_size / 1024 / 1024),
        ));
    }

    // Create a header bar with title
    let header_bar = adw::HeaderBar::new();
    header_bar.set_title_widget(Some(&adw::WindowTitle::new(&tr("Stream Info"), "")));

    // Create a toolbar view to combine header and content
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);

    let scrolled_window = gtk::ScrolledWindow::new();
    scrolled_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let whats_in_the_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    whats_in_the_box.set_margin_top(12);
    whats_in_the_box.set_margin_bottom(12);
    whats_in_the_box.set_margin_start(12);
    whats_in_the_box.set_margin_end(12);

    scrolled_window.set_child(Some(&whats_in_the_box));

    let local_list_box = create_listbox(&local_props);
    let local_label = gtk::Label::new(Some(&tr("Local Properties")));
    whats_in_the_box.append(&local_label);
    whats_in_the_box.append(&local_list_box);

    let remote_list_box = create_listbox(&remote_props);
    let remote_label = gtk::Label::new(Some(&tr("Remote Properties")));
    whats_in_the_box.append(&remote_label);
    whats_in_the_box.append(&remote_list_box);

    toolbar_view.set_content(Some(&scrolled_window));

    let dialog = adw::Dialog::builder()
        .can_close(true)
        .child(&toolbar_view)
        .build();
    dialog.set_content_width(300);
    dialog.set_content_height(600);
    dialog.present(parent);
}
