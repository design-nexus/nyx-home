//! Home: a line on what's going on, the scenes, a card per room with its
//! temperature, what's on or open and quick toggles, and the robots. Choosing
//! a room slides its devices in from the right (over the page in a narrow
//! window). Edit reorders and hides the sections, which include favorites,
//! energy and device health.
use crate::{
    app::{self, Home, Link},
    bar, cards, connection,
    device::{self, Device, Kind},
    grid::{self, CardGrid},
    ha::Entity,
    home_features::{self, RobotAction},
    icons, light_groups, prefs, present, robot_window, viewer,
};
use gtk::{gdk, glib, prelude::*};
use nyx_ui::widgets;
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
};

const DRAWER_WIDTH: i32 = 330;
const ROOM_CARD_WIDTH: i32 = 250;

pub struct Page {
    pub root: gtk::Widget,
    stack: gtk::Stack,
    empty: gtk::Box,
    body: gtk::Box,
    greeting: gtk::Label,
    line: gtk::Label,
    customize: gtk::ToggleButton,
    editing: Cell<bool>,
    lists: RefCell<HashMap<&'static str, gtk::Box>>,
    room_cards: RefCell<Vec<(String, gtk::Widget)>>,
    room: RefCell<Option<String>>,
    side: gtk::Revealer,
    side_line: gtk::Separator,
    veil: gtk::Box,
    veil_reveal: gtk::Revealer,
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
        let hello = widgets::hbox(12);
        hello.add_css_class("home-hello");
        let words = widgets::vbox(3);
        words.set_hexpand(true);
        let greeting = widgets::label("Good evening", "hello-title");
        greeting.set_accessible_role(gtk::AccessibleRole::Heading);
        let line = widgets::label("", "hello-line");
        line.set_wrap(true);
        words.append(&greeting);
        words.append(&line);
        hello.append(&words);
        let customize = bar::toggle(icons::PEN, "Edit");
        customize.add_css_class("flat");
        customize.set_tooltip_text(Some("Reorder or hide the sections on Home"));
        customize.connect_toggled(|button| {
            if let Some(app) = app::get() {
                app.overview.set_editing(button.is_active());
            }
        });
        hello.append(&customize);
        let page = widgets::vbox(0);
        page.append(&hello);
        page.append(&body);
        page.add_css_class("home-scroll-body");
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .hexpand(true)
            .child(&page)
            .build();

        // The room's drawer: beside the rooms, or over them when narrow.
        let motion = if prefs::get().reduce_motion { 0 } else { 200 };
        let side = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(motion)
            .hexpand(false)
            .build();
        let side_line = nyx_ui::frame::pane_line(gtk::Orientation::Vertical);
        side_line.set_visible(false);
        let row = widgets::hbox(0);
        row.append(&scroll);
        row.append(&side_line);
        row.append(&side);
        let veil = widgets::vbox(0);
        veil.add_css_class("home-veil");
        veil.set_visible(false);
        let veil_reveal = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(motion)
            .halign(gtk::Align::End)
            .hexpand(false)
            .vexpand(true)
            .build();
        veil.append(&veil_reveal);
        let click = gtk::GestureClick::new();
        {
            let (veil2, drawer) = (veil.clone(), veil_reveal.clone());
            click.connect_pressed(move |g, _, x, y| {
                let inside = veil2
                    .pick(x, y, gtk::PickFlags::DEFAULT)
                    .is_some_and(|w| w == drawer || w.is_ancestor(&drawer));
                if !inside {
                    g.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(app) = app::get() {
                        app.overview.close_room();
                    }
                }
            });
        }
        veil.add_controller(click);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&row));
        overlay.add_overlay(&veil);
        stack.add_named(&overlay, Some("content"));
        Self {
            root: stack.clone().upcast(),
            stack,
            empty,
            body,
            greeting,
            line,
            customize,
            editing: Cell::new(false),
            lists: RefCell::new(HashMap::new()),
            room_cards: RefCell::new(Vec::new()),
            room: RefCell::new(None),
            side,
            side_line,
            veil,
            veil_reveal,
            narrow: Cell::new(false),
            shown_link: RefCell::new(None),
        }
    }

    pub fn editing(&self) -> bool {
        self.editing.get()
    }

    pub fn set_editing(&self, on: bool) {
        if self.editing.replace(on) == on {
            return;
        }
        if self.customize.is_active() != on {
            self.customize.set_active(on);
        }
        app::rebuild();
    }

    /// Show a room's drawer beside the rooms, or over them when narrow.
    pub fn set_narrow(&self, narrow: bool) {
        if self.narrow.replace(narrow) == narrow {
            return;
        }
        if let Some(room) = self.room.borrow().clone() {
            self.place_drawer(&room);
        }
    }

    pub fn refresh(&self, home: &Home, changed: Option<&BTreeSet<String>>) {
        if !home.loaded() {
            if self.shown_link.borrow().as_ref() != Some(&home.link) {
                set_empty(&self.empty, &home.link);
                *self.shown_link.borrow_mut() = Some(home.link.clone());
            }
            self.stack.set_visible_child_name("empty");
            self.customize.set_sensitive(false);
            self.close_room();
            return;
        }
        *self.shown_link.borrow_mut() = None;
        self.customize.set_sensitive(true);
        self.stack.set_visible_child_name("content");
        self.greet(home);
        if changed.is_some() {
            // Cards and rooms keep themselves current; the lists are refilled.
            for (id, list) in self.lists.borrow().iter() {
                fill_list(id, list, home);
            }
            return;
        }
        self.build(home);
        if let Some(room) = self.room.borrow().clone() {
            self.place_drawer(&room);
        }
    }

    fn greet(&self, home: &Home) {
        let hour = glib::DateTime::now_local().map_or(12, |t| t.hour());
        self.greeting.set_text(match hour {
            5..12 => "Good morning",
            12..18 => "Good afternoon",
            _ => "Good evening",
        });
        self.line.set_text(&present::home_line(
            home.entities.values().filter(|e| !e.hidden),
        ));
    }

    fn build(&self, home: &Home) {
        clear(&self.body);
        self.lists.borrow_mut().clear();
        self.room_cards.borrow_mut().clear();
        migrate_favorites(home);
        let devices = home.device_list(false);
        let settings = prefs::get();
        let editing = self.editing.get();
        let order = section_order(&settings);
        if editing {
            let note = widgets::label(
                "Reorder sections with the arrows and turn off the ones you don't need. Drag cards to move them and use their size button to resize them. Press Esc or Edit when you're done.",
                "group-note",
            );
            note.set_wrap(true);
            note.set_margin_top(4);
            self.body.append(&note);
        }
        for (index, id) in order.iter().enumerate() {
            let hidden = settings.dashboard_hidden.contains(id);
            if hidden && !editing {
                continue;
            }
            let content = match id.as_str() {
                "scenes" => scenes(home),
                "rooms" => self.rooms(home, &devices),
                "robots" => robots(home, &devices),
                "favorites" => Some(favorites(home, &settings.favorite_devices, editing)),
                "energy" | "health" => {
                    let list = widgets::vbox(0);
                    list.add_css_class("group-list");
                    let id: &'static str = if id == "energy" { "energy" } else { "health" };
                    fill_list(id, &list, home);
                    let empty = list.first_child().is_none();
                    self.lists.borrow_mut().insert(id, list.clone());
                    (!empty).then(|| list.upcast())
                }
                _ => None,
            };
            if content.is_none() && !editing {
                continue;
            }
            if editing {
                self.body
                    .append(&section_head(id, index, order.len(), hidden));
            } else if id != "scenes" {
                self.body.append(&heading(id, &devices));
            }
            match content {
                Some(widget) => {
                    if hidden {
                        widget.add_css_class("dimmed");
                    }
                    self.body.append(&widget);
                }
                None => {
                    let note = widgets::label("Nothing to show yet.", "group-note");
                    self.body.append(&note);
                }
            }
        }
    }

    /// A card per room with devices, in Home Assistant's order.
    fn rooms(&self, home: &Home, devices: &[Device]) -> Option<gtk::Widget> {
        let flow = gtk::FlowBox::new();
        flow.add_css_class("room-cards");
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_homogeneous(true);
        flow.set_min_children_per_line(1);
        flow.set_max_children_per_line(4);
        flow.set_column_spacing(10);
        flow.set_row_spacing(10);
        flow.set_valign(gtk::Align::Start);
        flow.update_property(&[gtk::accessible::Property::Label("Rooms")]);
        let selected = self.room.borrow().clone();
        for area in &home.areas {
            let here: Vec<Device> = devices
                .iter()
                .filter(|d| d.area.as_ref() == Some(&area.name))
                .cloned()
                .collect();
            if here.is_empty() {
                continue;
            }
            let card = room_card(&area.name, here, home);
            if selected.as_ref() == Some(&area.name) {
                card.add_css_class("selected");
            }
            flow.insert(&card, -1);
            self.room_cards
                .borrow_mut()
                .push((area.name.clone(), card.upcast()));
        }
        let mut child = flow.first_child();
        while let Some(c) = child {
            c.set_focusable(false);
            child = c.next_sibling();
        }
        (flow.first_child().is_some()).then(|| flow.upcast())
    }

    /// Open `room`'s devices at the side.
    pub fn open_room(&self, room: &str) {
        *self.room.borrow_mut() = Some(room.to_string());
        for (name, card) in self.room_cards.borrow().iter() {
            if name == room {
                card.add_css_class("selected");
            } else {
                card.remove_css_class("selected");
            }
        }
        self.place_drawer(room);
    }

    /// Close the room's drawer; false if none was open.
    pub fn close_room(&self) -> bool {
        if self.room.borrow_mut().take().is_none() {
            return false;
        }
        for (_, card) in self.room_cards.borrow().iter() {
            card.remove_css_class("selected");
        }
        self.side.set_reveal_child(false);
        self.side_line.set_visible(false);
        self.veil_reveal.set_reveal_child(false);
        self.veil.set_visible(false);
        true
    }

    fn place_drawer(&self, room: &str) {
        let Some(app) = app::get() else { return };
        let home = app.home.borrow();
        let devices: Vec<Device> = home
            .device_list(false)
            .into_iter()
            .filter(|d| d.area.as_deref() == Some(room))
            .collect();
        if devices.is_empty() {
            drop(home);
            self.close_room();
            return;
        }
        let narrow = self.narrow.get();
        let drawer = drawer(room, &devices, &home);
        drawer.set_size_request(
            if narrow {
                DRAWER_WIDTH + 10
            } else {
                DRAWER_WIDTH
            },
            -1,
        );
        if narrow {
            drawer.add_css_class("floating");
            self.side.set_reveal_child(false);
            self.side_line.set_visible(false);
            self.veil_reveal.set_child(Some(&drawer));
            self.veil.set_visible(true);
            self.veil_reveal.set_reveal_child(true);
        } else {
            self.veil_reveal.set_reveal_child(false);
            self.veil.set_visible(false);
            self.side.set_child(Some(&drawer));
            self.side_line.set_visible(true);
            self.side.set_reveal_child(true);
        }
    }
}

/// A section's heading; Rooms links to every device.
fn heading(id: &str, devices: &[Device]) -> gtk::Box {
    let head = widgets::hbox(8);
    head.add_css_class("home-head");
    let title = widgets::label(section_title(id), "home-head-title");
    title.set_accessible_role(gtk::AccessibleRole::Heading);
    title.set_hexpand(true);
    head.append(&title);
    if id == "rooms" {
        let all = gtk::Button::with_label(&format!("All devices · {}", devices.len()));
        all.add_css_class("flat");
        all.add_css_class("home-link");
        all.set_tooltip_text(Some("Every device, by type"));
        all.connect_clicked(|_| app::open_room("", false));
        head.append(&all);
    }
    head
}

// ---------- Scenes ----------

/// The scenes and scripts as colored chips: a click runs one.
fn scenes(home: &Home) -> Option<gtk::Widget> {
    let p = prefs::get();
    let ids: Vec<String> = device::scenes(&home.entities)
        .into_values()
        .flatten()
        .filter(|id| !p.hidden_devices.contains(&format!("entity:{id}")))
        .collect();
    if ids.is_empty() {
        return None;
    }
    let flow = gtk::FlowBox::new();
    flow.add_css_class("scene-chips");
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_max_children_per_line(20);
    flow.set_column_spacing(8);
    flow.set_row_spacing(8);
    flow.set_halign(gtk::Align::Start);
    flow.set_valign(gtk::Align::Start);
    flow.update_property(&[gtk::accessible::Property::Label("Scenes")]);
    for id in grid::ordered(
        &p.card_order
            .get("rooms:scenes")
            .cloned()
            .unwrap_or_default(),
        ids,
    ) {
        flow.insert(&scene_chip(&id, home), -1);
    }
    let mut child = flow.first_child();
    while let Some(c) = child {
        c.set_focusable(false);
        child = c.next_sibling();
    }
    Some(flow.upcast())
}

fn scene_chip(id: &str, home: &Home) -> gtk::Button {
    let name = home
        .entities
        .get(id)
        .map(present::short_name)
        .unwrap_or_else(|| id.to_string());
    let b = gtk::Button::new();
    b.add_css_class("scene-chip");
    b.add_css_class(&format!("scene-color-{}", cards::scene_hue(id)));
    let content = widgets::hbox(8);
    content.append(&gtk::Image::from_icon_name(if id.starts_with("script.") {
        "system-run-symbolic"
    } else {
        icons::SCENE
    }));
    content.append(&gtk::Label::new(Some(&name)));
    b.set_child(Some(&content));
    b.set_tooltip_text(Some(&format!(
        "Run {name}. Right-click for its name and room."
    )));
    b.update_property(&[gtk::accessible::Property::Label(&format!("Run {name}"))]);
    let run = id.to_string();
    b.connect_clicked(move |_| {
        app::call(&run, "turn_on", json!({}));
        app::toast("Done.");
    });
    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_SECONDARY);
    let details = id.to_string();
    click.connect_pressed(move |_, _, _, _| crate::details::open(&details));
    b.add_controller(click);
    b
}

// ---------- Rooms ----------

/// A quick toggle on a room's card: lights, a plug, a speaker, the blinds.
struct Quick {
    icon: &'static str,
    tip: String,
    /// The entities that say whether it is on.
    ids: Vec<String>,
    on: fn(&Entity) -> bool,
    run: Box<dyn Fn(bool)>,
}

fn toggled(e: &Entity) -> bool {
    e.state == "on"
}

fn playing(e: &Entity) -> bool {
    e.state == "playing"
}

fn opened(e: &Entity) -> bool {
    e.state == "open"
}

/// Up to four quick toggles: the room's lights, then its plugs, fans, speakers
/// and blinds.
fn quick_toggles(room: &str, devices: &[Device], home: &Home) -> Vec<Quick> {
    let mut out = Vec::new();
    let lights: Vec<String> = devices
        .iter()
        .filter(|d| d.kind == Kind::Light && d.members.is_none())
        .filter_map(|d| d.primary.clone())
        .collect();
    if !lights.is_empty() {
        let (area, each) = (home.area_id(room), lights.clone());
        out.push(Quick {
            icon: icons::BULB,
            tip: format!("{room} lights"),
            ids: lights,
            on: toggled,
            run: Box::new(move |on| {
                let service = present::toggle_service(!on);
                match &area {
                    Some(area) => app::call_area(area, service, json!({})),
                    None => each.iter().for_each(|id| app::call(id, service, json!({}))),
                }
            }),
        });
    }
    for d in devices {
        if out.len() == 4 {
            break;
        }
        let Some(p) = d.primary(&home.entities) else {
            continue;
        };
        let id = p.entity_id.clone();
        let tip = present::without_room(&d.name, room);
        let quick = match (d.kind, p.domain()) {
            (Kind::Fan | Kind::Switch, "fan" | "switch" | "input_boolean" | "humidifier") => {
                Quick {
                    icon: d.kind.icon(),
                    tip,
                    ids: vec![id.clone()],
                    on: toggled,
                    run: Box::new(move |on| {
                        app::call(&id, present::toggle_service(!on), json!({}))
                    }),
                }
            }
            (Kind::Media, _) => Quick {
                icon: icons::SPEAKER,
                tip,
                ids: vec![id.clone()],
                on: playing,
                run: Box::new(move |_| app::call(&id, "media_play_pause", json!({}))),
            },
            (Kind::Cover, _) if !present::cover_actions(p).is_empty() => Quick {
                icon: icons::BLINDS,
                tip,
                ids: vec![id.clone()],
                on: opened,
                run: Box::new(move |on| {
                    app::call(
                        &id,
                        if on { "close_cover" } else { "open_cover" },
                        json!({}),
                    )
                }),
            },
            _ => continue,
        };
        out.push(quick);
    }
    out
}

/// A room's facts on one wrapping line: each fact stays whole, the ones that
/// need a look in the warning color.
fn facts_markup(facts: &[(String, bool)]) -> String {
    let warning = nyx_theme::tokens().color("status.warning").to_string();
    facts
        .iter()
        .map(|(text, warn)| {
            let text = glib::markup_escape_text(text).replace(' ', "\u{a0}");
            if *warn {
                format!("<span foreground=\"{warning}\" weight=\"500\">{text}</span>")
            } else {
                text
            }
        })
        .collect::<Vec<_>>()
        .join("\u{a0}\u{a0}<span alpha=\"50%\">·</span>  ")
}

fn room_card(room: &str, devices: Vec<Device>, home: &Home) -> gtk::Box {
    let (icon, swatch) = icons::room(room);
    let card: gtk::Box = glib::Object::builder()
        .property("orientation", gtk::Orientation::Vertical)
        .property("spacing", 10)
        .property("accessible-role", gtk::AccessibleRole::Button)
        .build();
    card.add_css_class("room-card");
    card.add_css_class(&format!("room-{swatch}"));
    card.set_focusable(true);
    card.set_size_request(ROOM_CARD_WIDTH, -1);
    let top = widgets::hbox(10);
    let chip = gtk::Image::from_icon_name(icon);
    chip.add_css_class("room-chip");
    chip.set_accessible_role(gtk::AccessibleRole::Presentation);
    top.append(&chip);
    let name = widgets::label(room, "room-name");
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    name.set_hexpand(true);
    top.append(&name);
    let temp = widgets::label("", "room-temp");
    top.append(&temp);
    card.append(&top);
    let facts = widgets::label("", "room-facts");
    facts.set_wrap(true);
    facts.set_lines(2);
    facts.set_ellipsize(gtk::pango::EllipsizeMode::End);
    facts.set_max_width_chars(28);
    facts.set_valign(gtk::Align::Start);
    card.append(&facts);
    let row = widgets::hbox(6);
    row.add_css_class("room-quick-row");
    row.set_vexpand(true);
    row.set_valign(gtk::Align::End);
    let mut quick_buttons = Vec::new();
    for q in quick_toggles(room, &devices, home) {
        let b = gtk::Button::from_icon_name(q.icon);
        b.add_css_class("room-quick");
        b.set_tooltip_text(Some(&q.tip));
        b.update_property(&[gtk::accessible::Property::Label(&q.tip)]);
        let state = std::rc::Rc::new(Cell::new(false));
        let s = state.clone();
        let run = q.run;
        b.connect_clicked(move |_| run(s.get()));
        row.append(&b);
        quick_buttons.push((b, q.ids, q.on, state));
    }
    if row.first_child().is_some() {
        card.append(&row);
    }
    let room_name = room.to_string();
    let ids: Vec<String> = devices.iter().flat_map(|d| d.entities.clone()).collect();
    let watched = devices.clone();
    let card_weak = card.downgrade();
    cards::watch(&card, ids, move |home| {
        let refs: Vec<&Device> = watched.iter().collect();
        let g = present::room_glance(&room_name, &refs, &home.entities);
        temp.set_text(&g.temperature.map(present::degrees).unwrap_or_default());
        facts.set_markup(&facts_markup(&g.facts));
        let spoken: Vec<&str> = g.facts.iter().map(|(text, _)| text.as_str()).collect();
        for (b, ids, on, state) in &quick_buttons {
            let lit = ids.iter().filter_map(|id| home.entities.get(id)).any(on);
            state.set(lit);
            if lit {
                b.add_css_class("on");
            } else {
                b.remove_css_class("on");
            }
        }
        if let Some(card) = card_weak.upgrade() {
            let temperature = g.temperature.map(present::degrees).unwrap_or_default();
            card.update_property(&[gtk::accessible::Property::Label(&format!(
                "{room_name} {temperature}: {}. Open its devices.",
                spoken.join(", ")
            ))]);
        }
    });
    let open = {
        let room = room.to_string();
        move || {
            if let Some(app) = app::get() {
                app.overview.open_room(&room);
            }
        }
    };
    let click = gtk::GestureClick::new();
    {
        let open = open.clone();
        click.connect_released(move |g, _, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            open();
        });
    }
    card.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    let me = card.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        let on_card = me.upgrade().is_some_and(|c| c.has_focus());
        if on_card && matches!(key, gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space) {
            open();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    card.add_controller(keys);
    card.set_tooltip_text(Some(&format!("Open {room}")));
    cards::notify_one(card.upcast_ref(), home);
    card
}

// ---------- The room's drawer ----------

fn drawer(room: &str, devices: &[Device], home: &Home) -> gtk::Box {
    let (icon, swatch) = icons::room(room);
    let panel = widgets::vbox(0);
    panel.set_hexpand(false);
    panel.add_css_class("room-drawer");
    panel.add_css_class(&format!("room-{swatch}"));
    panel.update_property(&[gtk::accessible::Property::Label(room)]);

    let head = widgets::vbox(12);
    head.add_css_class("drawer-head");
    let top = widgets::hbox(12);
    let chip = gtk::Image::from_icon_name(icon);
    chip.add_css_class("room-chip");
    chip.add_css_class("big");
    top.append(&chip);
    let words = widgets::vbox(1);
    words.set_hexpand(true);
    words.set_valign(gtk::Align::Center);
    let title = widgets::label(room, "drawer-title");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&title);
    let sub = widgets::label("", "drawer-sub");
    words.append(&sub);
    top.append(&words);
    let close = widgets::icon_button("window-close-symbolic", "Close (Esc)");
    close.connect_clicked(|_| {
        if let Some(app) = app::get() {
            app.overview.close_room();
        }
    });
    top.append(&close);
    head.append(&top);
    let scene_row = widgets::hbox(6);
    scene_row.add_css_class("drawer-scenes");
    let p = prefs::get();
    for id in device::scenes(&home.entities)
        .remove(room)
        .unwrap_or_default()
        .into_iter()
        .filter(|id| !p.hidden_devices.contains(&format!("entity:{id}")))
        .take(3)
    {
        let name = home
            .entities
            .get(&id)
            .map(present::short_name)
            .unwrap_or_else(|| id.clone());
        let b = bar::button(icons::SCENE, &present::without_room(&name, room));
        b.add_css_class("small");
        b.set_tooltip_text(Some(&format!("Run {name}")));
        b.connect_clicked(move |_| {
            app::call(&id, "turn_on", json!({}));
            app::toast("Done.");
        });
        scene_row.append(&b);
    }
    let lights_area = home.area_id(room);
    let has_lights = devices.iter().any(|d| d.kind == Kind::Light);
    if has_lights && let Some(area) = lights_area.clone() {
        let b = bar::button(icons::POWER, "Lights off");
        b.add_css_class("small");
        b.set_tooltip_text(Some(&format!("Turn off every light in {room}")));
        b.connect_clicked(move |_| app::call_area(&area, "turn_off", json!({})));
        scene_row.append(&b);
    }
    if scene_row.first_child().is_some() {
        head.append(&scene_row);
    }
    panel.append(&head);

    let body = widgets::vbox(0);
    body.add_css_class("drawer-body");
    let groups: [(&str, &[Kind]); 4] = [
        ("Lights", &[Kind::Light]),
        (
            "Climate and media",
            &[Kind::Climate, Kind::Fan, Kind::Media, Kind::Cover],
        ),
        (
            "Doors and security",
            &[Kind::Lock, Kind::Security, Kind::Camera],
        ),
        (
            "Everything else",
            &[Kind::Switch, Kind::Robot, Kind::Sensors],
        ),
    ];
    for (title, kinds) in groups {
        let mut list: Vec<&Device> = devices.iter().filter(|d| kinds.contains(&d.kind)).collect();
        list.sort_by_key(|d| (d.members.is_none(), d.kind));
        let room_group = (title == "Lights")
            .then(|| home.room_light_sets().into_iter().find(|(r, _)| r == room))
            .flatten();
        if list.is_empty() {
            continue;
        }
        let label_row = widgets::hbox(6);
        label_row.add_css_class("drawer-label-row");
        let label = widgets::label(title, "drawer-label");
        label.set_hexpand(true);
        label_row.append(&label);
        if title == "Lights" {
            let add = gtk::Button::new();
            add.add_css_class("flat");
            add.add_css_class("home-link");
            let content = widgets::hbox(4);
            content.append(&gtk::Image::from_icon_name("list-add-symbolic"));
            content.append(&gtk::Label::new(Some("Group")));
            add.set_child(Some(&content));
            add.set_tooltip_text(Some(&format!("New light group in {room}")));
            let r = room.to_string();
            add.connect_clicked(move |_| light_groups::create(Some(r.clone()), Vec::new()));
            label_row.append(&add);
        }
        body.append(&label_row);
        let card = widgets::vbox(0);
        card.add_css_class("drawer-card");
        if let Some((_, lights)) = &room_group {
            card.append(&room_lights_row(room, lights, lights_area.clone(), home));
        }
        for d in list {
            card.append(&device_row(d, room, home));
        }
        body.append(&card);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::External)
        .vexpand(true)
        .child(&body)
        .build();
    panel.append(&scroll);

    let foot = widgets::hbox(8);
    foot.add_css_class("drawer-foot");
    let open = bar::button(icons::EXPAND, "Open the room");
    open.set_tooltip_text(Some(&format!("Every card in {room}")));
    let r = room.to_string();
    open.connect_clicked(move |_| app::open_room(&r, false));
    foot.append(&open);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    foot.append(&spacer);
    let arrange = bar::button(icons::GRID, "Arrange");
    arrange.add_css_class("flat");
    arrange.set_tooltip_text(Some(&format!("Move and resize the cards in {room}")));
    let r = room.to_string();
    arrange.connect_clicked(move |_| app::open_room(&r, true));
    foot.append(&arrange);
    panel.append(&foot);

    let ids: Vec<String> = devices.iter().flat_map(|d| d.entities.clone()).collect();
    let (room_name, watched) = (room.to_string(), devices.to_vec());
    cards::watch(&panel, ids, move |home| {
        let refs: Vec<&Device> = watched.iter().collect();
        let g = present::room_glance(&room_name, &refs, &home.entities);
        let parts: Vec<String> = [
            g.temperature.map(present::degrees),
            g.humidity.map(|h| format!("{}%", h.round())),
            Some(present::plural(watched.len(), "device")),
        ]
        .into_iter()
        .flatten()
        .collect();
        sub.set_text(&parts.join(" · "));
    });
    cards::notify_one(panel.upcast_ref(), home);
    panel
}

/// The row parts every drawer row shares: a colored chip, the name and its
/// state, then the control. Clicking the row opens the device's details.
fn row_frame(kind: Kind, name: &str, open: impl Fn() + 'static) -> (gtk::Box, gtk::Label) {
    let row = widgets::hbox(12);
    row.add_css_class("drawer-row");
    row.add_css_class(kind.css());
    let chip = gtk::Image::from_icon_name(kind.icon());
    chip.add_css_class("chip");
    chip.set_valign(gtk::Align::Center);
    row.append(&chip);
    let words = widgets::vbox(1);
    words.set_hexpand(true);
    words.set_valign(gtk::Align::Center);
    let title = widgets::label(name, "row-name");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&title);
    let state = widgets::label("", "row-state");
    state.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&state);
    row.append(&words);
    let click = gtk::GestureClick::new();
    click.connect_released(move |g, _, _, _| {
        g.set_state(gtk::EventSequenceState::Claimed);
        open();
    });
    words.add_controller(click);
    words.set_tooltip_text(Some(&format!("{name}: details and settings")));
    (row, state)
}

fn set_on(row: &gtk::Box, on: bool) {
    if on {
        row.add_css_class("on");
    } else {
        row.remove_css_class("on");
    }
}

/// A room's lights as one, when no light group covers them.
fn room_lights_row(room: &str, lights: &[String], area: Option<String>, home: &Home) -> gtk::Box {
    let r = format!("{}{room}", cards::ROOM_LIGHTS);
    let (row, state) = row_frame(Kind::Light, &format!("{room} lights"), move || {
        crate::details::open(&r)
    });
    let sw = gtk::Switch::new();
    sw.set_valign(gtk::Align::Center);
    sw.update_property(&[gtk::accessible::Property::Label(&format!("{room} lights"))]);
    let each = lights.to_vec();
    let handler = sw.connect_state_set(move |_, on| {
        let service = present::toggle_service(on);
        match &area {
            Some(area) => app::call_area(area, service, json!({})),
            None => each.iter().for_each(|id| app::call(id, service, json!({}))),
        }
        glib::Propagation::Proceed
    });
    row.append(&sw);
    let (ids, r2) = (lights.to_vec(), row.clone());
    cards::watch(&row, ids.clone(), move |home| {
        let on: Vec<&Entity> = ids
            .iter()
            .filter_map(|id| home.entities.get(id))
            .filter(|e| e.state == "on")
            .collect();
        let level = on
            .iter()
            .filter_map(|e| present::brightness(e))
            .map(|b| b.round() as i64)
            .max();
        state.set_text(&match (on.len(), level) {
            (0, _) => format!("{} · off", present::plural(ids.len(), "light")),
            (n, Some(l)) => format!("{} on · {l}%", present::plural(n, "light")),
            (n, None) => format!("{} on", present::plural(n, "light")),
        });
        sw.block_signal(&handler);
        sw.set_active(!on.is_empty());
        sw.set_state(!on.is_empty());
        sw.unblock_signal(&handler);
        set_on(&r2, !on.is_empty());
    });
    cards::notify_one(row.upcast_ref(), home);
    row
}

/// Keeps a row's control in step with its entity.
type Setter = Box<dyn Fn(&Entity)>;

fn device_row(d: &Device, room: &str, home: &Home) -> gtk::Box {
    let id = d.id.clone();
    let (row, state) = row_frame(d.kind, &present::without_room(&d.name, room), move || {
        crate::details::open(&id)
    });
    let Some(p) = d.primary(&home.entities).cloned() else {
        return row;
    };
    let pid = p.entity_id.clone();
    let mut setter: Option<Setter> = None;
    match (d.kind, p.domain()) {
        (
            Kind::Light | Kind::Fan | Kind::Switch,
            "light" | "fan" | "switch" | "input_boolean" | "humidifier",
        ) => {
            let (sw, set) = cards::switch(&pid, &d.name);
            row.append(&sw);
            setter = Some(Box::new(move |e| set(e.state == "on")));
        }
        (Kind::Media, _) => {
            let b = widgets::icon_button("media-playback-start-symbolic", "Play or pause");
            let id = pid.clone();
            b.connect_clicked(move |_| app::call(&id, "media_play_pause", json!({})));
            row.append(&b);
            setter = Some(Box::new(move |e| {
                b.set_icon_name(if e.state == "playing" {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                })
            }));
        }
        (Kind::Cover, _) if !present::cover_actions(&p).is_empty() => {
            let b = widgets::icon_button("go-down-symbolic", "Close");
            let (id, shown) = (pid.clone(), std::rc::Rc::new(Cell::new(true)));
            let s = shown.clone();
            b.connect_clicked(move |_| {
                app::call(
                    &id,
                    if s.get() { "close_cover" } else { "open_cover" },
                    json!({}),
                )
            });
            row.append(&b);
            setter = Some(Box::new(move |e| {
                let open = e.state != "closed";
                shown.set(open);
                b.set_icon_name(if open {
                    "go-down-symbolic"
                } else {
                    "go-up-symbolic"
                });
                b.set_tooltip_text(Some(if open { "Close" } else { "Open" }));
            }));
        }
        (Kind::Lock, _) => {
            let id = pid.clone();
            let unlock = widgets::two_click("Unlock", "Unlock?", move || {
                app::call(&id, "unlock", json!({}))
            });
            unlock.remove_css_class("destructive-action");
            unlock.add_css_class("small");
            let lock = gtk::Button::with_label("Lock");
            lock.add_css_class("small");
            let id = pid.clone();
            lock.connect_clicked(move |_| app::call(&id, "lock", json!({})));
            row.append(&unlock);
            row.append(&lock);
            setter = Some(Box::new(move |e| {
                let locked = e.state == "locked";
                unlock.set_visible(locked);
                lock.set_visible(!locked);
            }));
        }
        (Kind::Camera, _) => {
            let b = widgets::icon_button(icons::PLAY, "Look");
            let (id, name) = (pid.clone(), d.name.clone());
            b.connect_clicked(move |_| viewer::open(&id, &name, None));
            row.append(&b);
        }
        (Kind::Robot, _) if home_features::robot_actions(&p).contains(&RobotAction::Start) => {
            let b = widgets::icon_button(icons::PLAY, RobotAction::Start.label(p.domain()));
            let id = pid.clone();
            b.connect_clicked(move |_| {
                if let Some(app) = app::get()
                    && let Some(e) = app.home.borrow().entities.get(&id).cloned()
                {
                    cards::run_robot(&e, RobotAction::Start);
                }
            });
            row.append(&b);
        }
        _ => {}
    }
    let r = row.clone();
    let kind = d.kind;
    cards::watch(&row, d.entities.clone(), move |home| {
        let Some(e) = home.entities.get(&pid) else {
            return;
        };
        state.set_text(&row_state(kind, e));
        set_on(
            &r,
            matches!(
                e.state.as_str(),
                "on" | "playing" | "open" | "heat" | "cleaning" | "mowing"
            ) || matches!(
                e.attributes["hvac_action"].as_str(),
                Some("heating" | "cooling")
            ),
        );
        if let Some(set) = &setter {
            set(e);
        }
    });
    cards::notify_one(row.upcast_ref(), home);
    row
}

/// A drawer row's state line: "55%", "Heating to 22°", "Rainy Day Jazz".
fn row_state(kind: Kind, e: &Entity) -> String {
    let a = &e.attributes;
    match kind {
        Kind::Light if e.state == "on" => match present::brightness(e) {
            Some(b) if b > 0.0 => format!("{}%", b.round()),
            _ => "On".into(),
        },
        Kind::Climate => match (a["hvac_action"].as_str(), a["temperature"].as_f64()) {
            (Some("heating"), Some(t)) => format!("Heating to {}", present::degrees(t)),
            (Some("cooling"), Some(t)) => format!("Cooling to {}", present::degrees(t)),
            _ => present::state_text(e),
        },
        Kind::Media => a["media_title"]
            .as_str()
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| present::state_text(e)),
        Kind::Cover => match (e.state.as_str(), a["current_position"].as_f64()) {
            ("open", Some(pos)) => format!("Open {}%", pos.round()),
            _ => present::state_text(e),
        },
        Kind::Fan => match (e.state.as_str(), a["preset_mode"].as_str()) {
            ("on", Some(mode)) => present::sentence_case(mode),
            _ => present::state_text(e),
        },
        _ => present::state_text(e),
    }
}

// ---------- Robots ----------

fn robots(home: &Home, devices: &[Device]) -> Option<gtk::Widget> {
    let robots: Vec<&Device> = devices.iter().filter(|d| d.kind == Kind::Robot).collect();
    if robots.is_empty() {
        return None;
    }
    let flow = gtk::FlowBox::new();
    flow.add_css_class("robot-rows");
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_homogeneous(true);
    flow.set_min_children_per_line(1);
    flow.set_max_children_per_line(2);
    flow.set_column_spacing(10);
    flow.set_row_spacing(10);
    flow.set_valign(gtk::Align::Start);
    flow.update_property(&[gtk::accessible::Property::Label("Robots")]);
    for d in robots {
        flow.insert(&robot_row(d, home), -1);
    }
    let mut child = flow.first_child();
    while let Some(c) = child {
        c.set_focusable(false);
        child = c.next_sibling();
    }
    Some(flow.upcast())
}

fn robot_row(d: &Device, home: &Home) -> gtk::Box {
    let row = widgets::hbox(12);
    row.add_css_class("robot-row");
    row.add_css_class(Kind::Robot.css());
    row.set_size_request(300, -1);
    let Some(r) = d.primary(&home.entities).cloned() else {
        return row;
    };
    let mower = r.domain() == "lawn_mower";
    let chip = gtk::Image::from_icon_name(if mower { icons::MOWER } else { icons::ROBOT });
    chip.add_css_class("chip");
    chip.set_valign(gtk::Align::Center);
    row.append(&chip);
    let words = widgets::vbox(1);
    words.set_hexpand(true);
    words.set_valign(gtk::Align::Center);
    let name = widgets::label(&d.name, "row-name");
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&name);
    let state = widgets::label("", "row-state");
    state.set_ellipsize(gtk::pango::EllipsizeMode::End);
    words.append(&state);
    row.append(&words);
    if home_features::robot_actions(&r).contains(&RobotAction::Start) {
        let start = bar::button(icons::PLAY, if mower { "Mow" } else { "Start" });
        start.add_css_class("small");
        start.set_tooltip_text(Some(RobotAction::Start.label(r.domain())));
        let id = r.entity_id.clone();
        start.connect_clicked(move |_| {
            if let Some(app) = app::get()
                && let Some(e) = app.home.borrow().entities.get(&id).cloned()
            {
                cards::run_robot(&e, RobotAction::Start);
            }
        });
        row.append(&start);
    }
    let map = bar::button(icons::PIN, "Map");
    map.add_css_class("small");
    map.add_css_class("flat");
    map.set_tooltip_text(Some("Live map and every command"));
    let id = d.id.clone();
    map.connect_clicked(move |_| robot_window::open(&id));
    row.append(&map);
    let battery = d
        .find(&home.entities, |e| {
            e.domain() == "sensor" && device::device_class(e) == "battery"
        })
        .map(|e| e.entity_id.clone());
    let (id, r2) = (r.entity_id.clone(), row.clone());
    cards::watch(&row, d.entities.clone(), move |home| {
        let Some(r) = home.entities.get(&id) else {
            return;
        };
        let level = battery
            .as_ref()
            .and_then(|b| home.entities.get(b))
            .and_then(|e| e.state.parse::<f64>().ok())
            .or_else(|| r.attributes["battery_level"].as_f64());
        state.set_text(&match level {
            Some(l) => format!("{} · battery {}%", present::state_text(r), l.round()),
            None => present::state_text(r),
        });
        set_on(
            &r2,
            matches!(r.state.as_str(), "cleaning" | "mowing" | "returning"),
        );
    });
    cards::notify_one(row.upcast_ref(), home);
    row
}

// ---------- Favorites, energy and health ----------

/// Cards for `devices` on the card grid, in their saved order.
fn grid_of<'a>(
    home: &Home,
    key: &str,
    devices: impl Iterator<Item = &'a Device>,
    editing: bool,
) -> Option<gtk::Widget> {
    let list: Vec<&Device> = devices.collect();
    if list.is_empty() {
        return None;
    }
    let g = CardGrid::new(
        key,
        cards::maker(list.iter().map(|d| (*d).clone()).collect(), true),
    );
    for d in &list {
        cards::add_to(&g, &d.id, cards::size_options(d, home));
    }
    g.set_arranging(editing);
    Some(g.upcast())
}

/// Pinned devices; a light its group hides still shows here.
fn favorites(home: &Home, favorites: &[String], editing: bool) -> gtk::Widget {
    let mut devices = home.device_list(true);
    devices.retain(|d| !d.hidden);
    let pinned: Vec<&Device> = favorites
        .iter()
        .filter_map(|id| devices.iter().find(|d| d.id == *id))
        .collect();
    if let Some(grid) = grid_of(home, "overview:favorites", pinned.into_iter(), editing) {
        return grid;
    }
    let row = widgets::hbox(12);
    row.add_css_class("card");
    row.add_css_class("hint-card");
    let text = widgets::label(
        "Pin the devices you use most. Open a device's details and choose Add to Home.",
        "dim",
    );
    text.set_wrap(true);
    text.set_hexpand(true);
    row.append(&text);
    let open = gtk::Button::with_label("All devices");
    open.set_valign(gtk::Align::Center);
    open.connect_clicked(|_| app::open_room("", false));
    row.append(&open);
    row.upcast()
}

/// Favorites used to be entities; pin their devices instead, once.
fn migrate_favorites(home: &Home) {
    let old = prefs::get().favorite_entities;
    if old.is_empty() || home.entities.is_empty() {
        return;
    }
    let ids: Vec<String> = old
        .iter()
        .filter_map(|e| home.entities.get(e))
        .map(|e| {
            e.device_id
                .clone()
                .unwrap_or_else(|| format!("entity:{}", e.entity_id))
        })
        .collect();
    prefs::update(|p| {
        for id in ids {
            if !p.favorite_devices.contains(&id) {
                p.favorite_devices.push(id);
            }
        }
        p.favorite_entities.clear();
    });
}

fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

/// The saved order of Home's sections; sections it doesn't have yet go
/// where they are by default.
fn section_order(settings: &prefs::Prefs) -> Vec<String> {
    let mut order: Vec<String> = settings
        .dashboard_order
        .iter()
        .filter(|id| {
            home_features::DASHBOARD_CARDS
                .iter()
                .any(|(card, _)| card == id)
        })
        .cloned()
        .collect();
    for (i, (id, _)) in home_features::DASHBOARD_CARDS.iter().enumerate() {
        if !order.iter().any(|saved| saved == id) {
            order.insert(i.min(order.len()), id.to_string());
        }
    }
    order
}

fn section_title(id: &str) -> &'static str {
    home_features::DASHBOARD_CARDS
        .iter()
        .find(|(card, _)| *card == id)
        .map_or("Section", |(_, name)| name)
}

fn section_head(id: &str, index: usize, count: usize, hidden: bool) -> gtk::Box {
    let head = widgets::hbox(4);
    head.add_css_class("group-head");
    let title = widgets::label(&section_title(id).to_uppercase(), "group-title");
    title.set_hexpand(true);
    head.append(&title);
    let name = section_title(id);
    for (icon, tip, step, enabled) in [
        ("go-up-symbolic", "Move up", -1isize, index > 0),
        ("go-down-symbolic", "Move down", 1, index + 1 < count),
    ] {
        let button = widgets::icon_button(icon, &format!("{tip}: {name}"));
        button.set_sensitive(enabled);
        let id = id.to_string();
        button.connect_clicked(move |_| {
            prefs::update(|p| {
                let mut order = section_order(p);
                if let Some(at) = order.iter().position(|o| *o == id) {
                    let to = (at as isize + step).clamp(0, order.len() as isize - 1) as usize;
                    order.swap(at, to);
                }
                p.dashboard_order = order;
            });
            app::rebuild();
        });
        head.append(&button);
    }
    let show = gtk::Switch::new();
    show.set_active(!hidden);
    show.set_valign(gtk::Align::Center);
    show.set_tooltip_text(Some(&format!("Show {name} on Home")));
    show.update_property(&[gtk::accessible::Property::Label(&format!("Show {name}"))]);
    let id = id.to_string();
    show.connect_active_notify(move |switch| {
        let visible = switch.is_active();
        prefs::update(|p| {
            p.dashboard_hidden.retain(|h| *h != id);
            if !visible {
                p.dashboard_hidden.push(id.clone());
            }
        });
        app::rebuild();
    });
    head.append(&show);
    head
}

fn fill_list(id: &str, list: &gtk::Box, home: &Home) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    match id {
        "energy" => {
            for sensor in home
                .entities
                .values()
                .filter(|e| home_features::is_energy(e) && !e.hidden)
            {
                let value = widgets::label(&present::state_text(sensor), "num");
                let row = widgets::row(
                    &sensor.name(),
                    &glib_escape(sensor.area_name.as_deref().unwrap_or("")),
                    Some(value.upcast_ref()),
                );
                list.append(&row);
            }
        }
        _ => {
            let issues = present::health(home.entities.values());
            let total = issues.len();
            for issue in issues.into_iter().take(10) {
                let tag = widgets::tag(&issue.problem, "warning");
                list.append(&widgets::row(
                    &issue.title,
                    &glib_escape(&issue.place),
                    Some(tag.upcast_ref()),
                ));
            }
            if total > 10 {
                list.append(&widgets::row(
                    &format!("{} more", total - 10),
                    "Search for them in All devices.",
                    None,
                ));
            }
        }
    }
}

fn glib_escape(text: &str) -> String {
    gtk::glib::markup_escape_text(text).to_string()
}

/// What to show instead of devices while Home Assistant isn't loaded.
pub fn set_empty(container: &gtk::Box, link: &Link) {
    clear(container);
    let action = |label: &'static str, run: fn()| -> nyx_ui::widgets::Action<'static> {
        Some((label, Box::new(run)))
    };
    let escaped = |text: &str| glib_escape(text);
    let state = match link {
        Link::NotSetUp => widgets::empty_state(
            "network-server-symbolic",
            "Connect Home Assistant",
            "Home shows your rooms, devices and automations from your Home Assistant server.",
            action("Connect…", connection::dialog),
        ),
        Link::Unlocking => widgets::empty_state(
            "dialog-password-symbolic",
            "Unlocking your keyring…",
            "Home keeps its Home Assistant token in your desktop keyring. Unlock it if you're asked.",
            None,
        ),
        Link::Connecting | Link::Online | Link::Offline(_) => {
            widgets::empty_state("network-wireless-symbolic", "Loading your home…", "", None)
        }
        Link::NeedsToken => widgets::empty_state(
            "dialog-password-symbolic",
            "Add an access token",
            "Home Assistant is set up, but no token is saved for it.",
            action("Add token…", connection::dialog),
        ),
        Link::Rejected => widgets::empty_state(
            "dialog-password-symbolic",
            "Home Assistant rejected the token",
            "The token may have been deleted or expired. Create a new one in your Home Assistant profile.",
            action("Replace token…", connection::dialog),
        ),
        Link::Locked(message) => widgets::empty_state(
            "dialog-password-symbolic",
            "Unlock your keyring",
            &escaped(message),
            action("Try again", connection::restore),
        ),
        Link::Unreachable(message) => widgets::empty_state(
            "network-offline-symbolic",
            "Can't reach Home Assistant",
            &format!("{} Home keeps trying.", escaped(message)),
            action("Check connection…", connection::dialog),
        ),
    };
    container.append(&state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sections_take_their_default_place() {
        let p = prefs::Prefs {
            dashboard_order: [
                "summary",
                "favorites",
                "cameras",
                "robots",
                "energy",
                "health",
            ]
            .map(String::from)
            .to_vec(),
            ..Default::default()
        };
        assert_eq!(
            section_order(&p),
            ["scenes", "rooms", "favorites", "robots", "energy", "health"]
        );
    }
}
