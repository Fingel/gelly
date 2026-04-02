use gtk::{graphene::Point, prelude::*};

pub fn album_art_widget_snapshot(
    snapshot: &gtk::Snapshot,
    paintable: Option<&gtk::gdk::Paintable>,
    width: f64,
    height: f64,
    translate: Option<(f32, f32)>,
) {
    if let Some(texture) = paintable {
        snapshot.push_opacity(0.15);
        snapshot.push_blur(80.0);
        if let Some((tx, ty)) = translate {
            snapshot.translate(&Point::new(tx, ty));
        }
        texture.snapshot(snapshot, width, height);
        snapshot.pop();
        snapshot.pop();
    }
}
