//! The bar across the top of a page: its title, a line about it, and its
//! actions as labelled buttons. Home itself has none; it opens on a greeting.
use gtk::prelude::*;
use nyx_ui::widgets;

pub struct PageBar {
    pub root: gtk::Box,
    /// Back to Home, for the pages Home opens (a room, Connections).
    pub back: gtk::Button,
    pub title: gtk::Label,
    pub subtitle: gtk::Label,
    pub actions: gtk::Box,
}

impl PageBar {
    pub fn new(title: &str) -> Self {
        let root = widgets::hbox(8);
        root.add_css_class("page-bar");
        let back = widgets::bar_button("go-previous-symbolic", "Back to Home (Alt+←)");
        back.update_property(&[gtk::accessible::Property::Label("Back to Home")]);
        back.connect_clicked(|_| crate::app::show(crate::app::Section::Overview));
        back.set_visible(false);
        root.append(&back);
        let title = widgets::label(title, "page-title");
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        root.append(&title);
        let subtitle = widgets::label("", "page-subtitle");
        subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
        subtitle.set_hexpand(true);
        root.append(&subtitle);
        let actions = widgets::hbox(6);
        actions.set_valign(gtk::Align::Center);
        root.append(&actions);
        Self {
            root,
            back,
            title,
            subtitle,
            actions,
        }
    }

    /// The bar over `content`, as the page's root.
    pub fn over(&self, content: &impl IsA<gtk::Widget>) -> gtk::Widget {
        let page = widgets::vbox(0);
        page.append(&self.root);
        content.set_vexpand(true);
        page.append(content);
        page.upcast()
    }
}

/// A button with an icon and its name.
pub fn button(icon: &str, label: &str) -> gtk::Button {
    let b = widgets::labeled_button(icon, label);
    b.set_valign(gtk::Align::Center);
    b
}

/// A toggle with an icon and its name.
pub fn toggle(icon: &str, label: &str) -> gtk::ToggleButton {
    let b = gtk::ToggleButton::new();
    let content = widgets::hbox(8);
    content.append(&gtk::Image::from_icon_name(icon));
    content.append(&gtk::Label::new(Some(label)));
    b.set_child(Some(&content));
    b.set_valign(gtk::Align::Center);
    b
}
