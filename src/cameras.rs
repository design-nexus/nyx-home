//! Cameras: a wall of every camera's last picture, the first one large, each
//! with its room, what it last saw, its power and Live. Arrange turns the wall
//! into cards to drag; the wall follows their order.
use crate::{
    app::{self, Home, Link},
    bar::PageBar,
    cards,
    device::{Device, Kind},
    grid::{self, CardGrid, Sections},
    icons, overview, prefs, present, rooms, viewer,
};
use gtk::prelude::*;
use nyx_ui::widgets;
use std::cell::{Cell, RefCell};

const KEY: &str = "cameras:all";
/// The height of a small tile; the first spans two rows and two columns.
const TILE_HEIGHT: i32 = 172;

pub struct Page {
    pub root: gtk::Widget,
    bar: PageBar,
    stack: gtk::Stack,
    empty: gtk::Box,
    wall: gtk::Grid,
    tiles: RefCell<Vec<gtk::Widget>>,
    note: gtk::Label,
    flow: Sections,
    arrange: gtk::ToggleButton,
    reset: gtk::Button,
    arranging: Cell<bool>,
    narrow: Cell<bool>,
    shown_link: RefCell<Option<Link>>,
}

impl Page {
    pub fn new() -> Self {
        let stack = gtk::Stack::new();
        let empty = widgets::vbox(0);
        empty.set_vexpand(true);
        stack.add_named(&empty, Some("empty"));

        let body = widgets::vbox(0);
        body.add_css_class("settings-page");
        body.add_css_class("home-page");
        let wall = gtk::Grid::new();
        wall.add_css_class("camera-wall");
        wall.set_column_homogeneous(true);
        wall.set_row_homogeneous(true);
        wall.set_column_spacing(10);
        wall.set_row_spacing(10);
        wall.update_property(&[gtk::accessible::Property::Label("Cameras")]);
        body.append(&wall);
        let note_row = widgets::hbox(8);
        note_row.add_css_class("camera-note");
        let info = gtk::Image::from_icon_name("dialog-information-symbolic");
        info.set_valign(gtk::Align::Start);
        note_row.append(&info);
        let note = widgets::label("", "");
        note.set_wrap(true);
        note.set_hexpand(true);
        note_row.append(&note);
        body.append(&note_row);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&body)
            .build();
        stack.add_named(&scroll, Some("wall"));

        let cards_body = widgets::vbox(0);
        cards_body.add_css_class("settings-page");
        cards_body.add_css_class("home-page");
        let hint = widgets::label(
            "Drag the cameras into the order you want; the first one is shown large. Press Esc or Arrange when you're done.",
            "group-note",
        );
        hint.set_wrap(true);
        hint.set_margin_top(4);
        cards_body.append(&hint);
        let flow = Sections::new("cameras");
        cards_body.append(&flow);
        let cards_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&cards_body)
            .build();
        stack.add_named(&cards_scroll, Some("arrange"));

        let bar = PageBar::new("Cameras");
        let (arrange, reset) = rooms::arrange_buttons(&bar.actions);
        arrange.set_tooltip_text(Some("Put the cameras in your order"));
        arrange.connect_toggled(|b| {
            if let Some(app) = app::get() {
                app.cameras.set_arranging(b.is_active());
            }
        });
        reset.connect_clicked(|_| {
            if let Some(app) = app::get() {
                let ids: Vec<String> = app
                    .cameras
                    .flow
                    .grids()
                    .iter()
                    .flat_map(|g| g.ids())
                    .collect();
                grid::reset("cameras", &ids);
                app::rebuild();
            }
        });
        rooms::escape_leaves(&stack, &arrange);
        Self {
            root: bar.over(&stack),
            bar,
            stack,
            empty,
            wall,
            tiles: RefCell::new(Vec::new()),
            note,
            flow,
            arrange,
            reset,
            arranging: Cell::new(false),
            narrow: Cell::new(false),
            shown_link: RefCell::new(None),
        }
    }

    fn set_arranging(&self, on: bool) {
        if self.arranging.replace(on) == on {
            return;
        }
        self.flow.set_arranging(on);
        self.reset.set_visible(on);
        if self.arrange.is_active() != on {
            self.arrange.set_active(on);
        }
        app::rebuild();
    }

    /// Two columns in a narrow window, three otherwise.
    pub fn set_narrow(&self, narrow: bool) {
        if self.narrow.replace(narrow) != narrow {
            self.place();
        }
    }

    pub fn refresh(&self, home: &Home, changed: bool) {
        if !home.loaded() {
            if self.shown_link.borrow().as_ref() != Some(&home.link) {
                overview::set_empty(&self.empty, &home.link);
                *self.shown_link.borrow_mut() = Some(home.link.clone());
            }
            self.bar.subtitle.set_text("");
            self.stack.set_visible_child_name("empty");
            return;
        }
        *self.shown_link.borrow_mut() = None;
        if changed {
            return;
        }
        let devices = home.device_list(false);
        let mut cameras: Vec<&Device> = devices.iter().filter(|d| d.kind == Kind::Camera).collect();
        if cameras.is_empty() {
            while let Some(child) = self.empty.first_child() {
                self.empty.remove(&child);
            }
            self.empty.append(&widgets::empty_state(
                icons::CAMERA,
                "No cameras",
                "Cameras you add to Home Assistant show up here with their last picture.",
                None,
            ));
            self.bar.subtitle.set_text("");
            self.stack.set_visible_child_name("empty");
            return;
        }
        cameras.sort_by_key(|d| (d.area.clone().unwrap_or_default(), d.name.to_lowercase()));
        let saved = prefs::get()
            .card_order
            .get(KEY)
            .cloned()
            .unwrap_or_default();
        let order = grid::ordered(&saved, cameras.iter().map(|d| d.id.clone()).collect());
        let cameras: Vec<&Device> = order
            .iter()
            .filter_map(|id| cameras.iter().copied().find(|d| d.id == *id))
            .collect();
        self.bar.subtitle.set_text(&format!(
            "{} · stills from the last check",
            present::plural(cameras.len(), "camera")
        ));
        self.note.set_text(&match prefs::get().camera_ask_seconds {
            0 => "Stills load once and don't refresh on their own, so battery cameras stay asleep. Live video plays until you stop it.".to_string(),
            s => format!(
                "Stills load once and don't refresh on their own, so battery cameras stay asleep. Live video asks if you're still watching after {}.",
                present::plural((s as usize).div_ceil(60), "minute")
            ),
        });
        if self.arranging.get() {
            self.flow.clear();
            let maker = cards::maker(cameras.iter().map(|d| (*d).clone()).collect(), true);
            let g = CardGrid::new(KEY, maker);
            for d in &cameras {
                cards::add_to(&g, &d.id, cards::size_options(d, home));
            }
            g.update_property(&[gtk::accessible::Property::Label("Cameras")]);
            self.flow.add_plain(KEY, &g);
            self.flow.set_arranging(true);
            self.stack.set_visible_child_name("arrange");
            return;
        }
        let tiles: Vec<gtk::Widget> = cameras
            .iter()
            .enumerate()
            .map(|(i, d)| tile(d, home, i == 0))
            .collect();
        *self.tiles.borrow_mut() = tiles;
        self.place();
        self.stack.set_visible_child_name("wall");
    }

    /// Lay the tiles out: the first two columns wide and two rows tall, the
    /// rest filling the gaps in order.
    fn place(&self) {
        while let Some(child) = self.wall.first_child() {
            self.wall.remove(&child);
        }
        let cols = if self.narrow.get() { 2 } else { 3 };
        let mut taken: Vec<Vec<bool>> = Vec::new();
        for (i, tile) in self.tiles.borrow().iter().enumerate() {
            let span = if i == 0 { 2 } else { 1 };
            let mut row = 0;
            let (c, r) = loop {
                while taken.len() < row + span {
                    taken.push(vec![false; cols]);
                }
                let free = (0..=cols - span)
                    .find(|&c| (row..row + span).all(|r| (c..c + span).all(|cc| !taken[r][cc])));
                if let Some(c) = free {
                    break (c, row);
                }
                row += 1;
            };
            for line in taken.iter_mut().skip(r).take(span) {
                line[c..c + span].fill(true);
            }
            self.wall
                .attach(tile, c as i32, r as i32, span as i32, span as i32);
        }
    }
}

/// A camera's tile: its last picture with its name, room and what it last
/// saw over it, its power, and Live for cameras that stream.
fn tile(d: &Device, home: &Home, big: bool) -> gtk::Widget {
    let overlay = gtk::Overlay::new();
    overlay.add_css_class("camera-tile");
    overlay.set_overflow(gtk::Overflow::Hidden);
    overlay.set_size_request(-1, TILE_HEIGHT);
    let Some(cam) = d.primary(&home.entities).cloned() else {
        return overlay.upcast();
    };
    let base = widgets::vbox(0);
    base.add_css_class("camera-blank");
    base.add_css_class(&format!("scene-color-{}", cards::scene_hue(&cam.entity_id)));
    let icon = gtk::Image::from_icon_name(icons::CAMERA);
    icon.set_pixel_size(if big { 36 } else { 28 });
    icon.set_vexpand(true);
    icon.set_valign(gtk::Align::Center);
    base.append(&icon);
    overlay.set_child(Some(&base));
    overlay.add_overlay(&viewer::still(&cam.entity_id));

    let shade = widgets::hbox(8);
    shade.add_css_class("camera-shade");
    shade.set_valign(gtk::Align::End);
    let words = widgets::vbox(1);
    words.set_hexpand(true);
    words.set_valign(gtk::Align::End);
    let name = widgets::label(&d.name, "camera-name");
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&name);
    let line = widgets::label("", "camera-line");
    line.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&line);
    shade.append(&words);
    let power = widgets::label("", "camera-power");
    power.set_valign(gtk::Align::End);
    shade.append(&power);
    if viewer::streams(&cam) {
        let live = crate::bar::button(icons::PLAY, if big { "Watch live" } else { "Live" });
        live.add_css_class("small");
        live.add_css_class(if big {
            "suggested-action"
        } else {
            "camera-live"
        });
        live.set_valign(gtk::Align::End);
        live.set_tooltip_text(Some(&format!("Live video from {}", d.name)));
        let id = cam.entity_id.clone();
        live.connect_clicked(move |_| viewer::open_live(&id));
        shade.append(&live);
    }
    overlay.add_overlay(&shade);

    let (id, title) = (cam.entity_id.clone(), d.name.clone());
    let click = gtk::GestureClick::new();
    click.connect_released(move |g, _, _, _| {
        g.set_state(gtk::EventSequenceState::Claimed);
        viewer::open(&id, &title, None);
    });
    overlay.add_controller(click);
    overlay.set_tooltip_text(Some(&format!("Open {} larger", d.name)));
    overlay.set_accessible_role(gtk::AccessibleRole::Button);

    let (device, room) = (d.clone(), d.area.clone());
    let weak = overlay.downgrade();
    cards::watch(&overlay, d.entities.clone(), move |home| {
        let unavailable = home
            .entities
            .get(&cam.entity_id)
            .is_none_or(present::unavailable);
        let event = if unavailable {
            "Unavailable".to_string()
        } else {
            cards::camera_event(&device, home).unwrap_or_else(|| "Quiet".into())
        };
        let text = match &room {
            Some(room) => format!("{room} · {event}"),
            None => event,
        };
        line.set_text(&text);
        let p = cards::camera_power(&device, home).unwrap_or_default();
        power.set_text(&p);
        power.set_visible(!p.is_empty());
        if let Some(o) = weak.upgrade() {
            o.update_property(&[gtk::accessible::Property::Label(&format!(
                "{}, {text}",
                device.name
            ))]);
        }
    });
    cards::notify_one(overlay.upcast_ref(), home);
    overlay.upcast()
}
