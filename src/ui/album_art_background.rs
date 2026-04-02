use gtk::{graphene, prelude::*};

pub fn create_blur_paintable(
    widget: &impl IsA<gtk::Widget>,
    paintable: &gtk::gdk::Paintable,
    width: i32,
    height: i32,
) -> Option<gtk::gdk::Paintable> {
    let snapshot = gtk::Snapshot::new();
    snapshot.push_opacity(0.15);
    snapshot.push_blur(80.0);
    paintable.snapshot(&snapshot, width as f64, height as f64);
    snapshot.pop();
    snapshot.pop();

    let rect = graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
    let node = snapshot.to_node()?;
    let texture = widget
        .upcast_ref::<gtk::Widget>()
        .native()?
        .renderer()?
        .render_texture(&node, Some(&rect));
    Some(texture.upcast())
}

pub fn draw_background(
    snapshot: &gtk::Snapshot,
    paintable: &gtk::gdk::Paintable,
    width: f64,
    height: f64,
    translate: Option<(f32, f32)>,
) {
    if let Some((tx, ty)) = translate {
        snapshot.save();
        snapshot.translate(&graphene::Point::new(tx, ty));
        paintable.snapshot(snapshot, width, height);
        snapshot.restore();
    } else {
        paintable.snapshot(snapshot, width, height);
    }
}
