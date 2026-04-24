use adw::prelude::AnimationExt;
use gtk::{glib::WeakRef, graphene, prelude::*};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct BlurBackground {
    paintable: Rc<RefCell<Option<gtk::gdk::Paintable>>>,
    prev_paintable: Rc<RefCell<Option<gtk::gdk::Paintable>>>,
    fade_alpha: Rc<Cell<f64>>,
    animation: RefCell<Option<adw::TimedAnimation>>,
    targets: Rc<RefCell<Vec<WeakRef<gtk::Widget>>>>,
}

impl Default for BlurBackground {
    fn default() -> Self {
        Self {
            paintable: Rc::new(RefCell::new(None)),
            prev_paintable: Rc::new(RefCell::new(None)),
            fade_alpha: Rc::new(Cell::new(1.0)),
            animation: RefCell::new(None),
            targets: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl BlurBackground {
    pub fn has_content(&self) -> bool {
        self.paintable.borrow().is_some()
    }

    pub fn add_draw_target(&self, widget: &impl IsA<gtk::Widget>) {
        self.targets
            .borrow_mut()
            .push(widget.upcast_ref::<gtk::Widget>().downgrade());
    }

    pub fn update(&self, new_paintable: Option<gtk::gdk::Paintable>) {
        let prev = self.paintable.borrow().clone();
        *self.prev_paintable.borrow_mut() = prev;
        *self.paintable.borrow_mut() = new_paintable;
        self.start_fade();
    }

    fn start_fade(&self) {
        let Some(widget) = self.targets.borrow().iter().find_map(|w| w.upgrade()) else {
            return;
        };

        self.fade_alpha.set(0.0);

        let fade_alpha = self.fade_alpha.clone();
        let targets = self.targets.clone();

        let target = adw::CallbackAnimationTarget::new({
            let fade_alpha = fade_alpha.clone();
            let targets = targets.clone();
            move |value| {
                fade_alpha.set(value);
                for weak in targets.borrow().iter() {
                    if let Some(w) = weak.upgrade() {
                        w.queue_draw();
                    }
                }
            }
        });

        let animation = adw::TimedAnimation::new(&widget, 0.0, 1.0, 500, target);
        animation.set_easing(adw::Easing::EaseOut);

        let prev_paintable = self.prev_paintable.clone();
        animation.connect_done(move |_| {
            fade_alpha.set(1.0);
            *prev_paintable.borrow_mut() = None;
            for weak in targets.borrow().iter() {
                if let Some(w) = weak.upgrade() {
                    w.queue_draw();
                }
            }
        });

        animation.play();
        *self.animation.borrow_mut() = Some(animation);
    }

    pub fn snapshot(
        &self,
        snapshot: &gtk::Snapshot,
        width: f64,
        height: f64,
        translate: Option<(f32, f32)>,
    ) {
        let alpha = self.fade_alpha.get();
        if alpha < 1.0
            && let Some(p) = self.prev_paintable.borrow().as_ref()
        {
            snapshot.push_opacity(1.0 - alpha);
            draw_background(snapshot, p, width, height, translate);
            snapshot.pop();
        }
        if let Some(p) = self.paintable.borrow().as_ref() {
            snapshot.push_opacity(alpha);
            draw_background(snapshot, p, width, height, translate);
            snapshot.pop();
        }
    }
}

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
