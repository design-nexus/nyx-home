//! Developer aid: `NYX_HOME_DEMO=<file.json>` shows made-up devices from a file
//! instead of Home Assistant, for screenshots. Nothing is read from the keyring
//! and nothing is sent anywhere: the token stays empty, so every control stops
//! at "Connect Home Assistant first", and the desktop service isn't called.
//! Camera stills are `<entity_id>.png` beside the file.
use crate::{
    app::{self, Link},
    ha,
};
use gtk::gdk;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize)]
struct Fixture {
    location_name: String,
    areas: Vec<String>,
    #[serde(default)]
    devices: Vec<FixtureDevice>,
    entities: Vec<FixtureEntity>,
}

#[derive(Deserialize)]
struct FixtureDevice {
    id: String,
    name: String,
    area: Option<String>,
    maker: Option<String>,
    model: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
}

#[derive(Deserialize)]
struct FixtureEntity {
    entity_id: String,
    state: String,
    #[serde(default)]
    attributes: Value,
    device: Option<String>,
    area: Option<String>,
    category: Option<String>,
    #[serde(default)]
    minutes_ago: i64,
}

fn path() -> Option<PathBuf> {
    std::env::var_os("NYX_HOME_DEMO").map(PathBuf::from)
}

pub fn active() -> bool {
    path().is_some()
}

/// A camera's still from beside the fixture.
pub fn picture(entity_id: &str) -> Result<gdk::Texture, String> {
    let dir = path()
        .and_then(|p| p.parent().map(PathBuf::from))
        .ok_or("No demo folder.")?;
    gdk::Texture::from_filename(dir.join(format!("{entity_id}.png")))
        .map_err(|_| "No picture yet.".to_string())
}

/// Fill Home with the fixture and show it as connected.
pub fn start() {
    let Some(app) = app::get() else { return };
    let Some(file) = path() else { return };
    let fixture: Fixture = match std::fs::read_to_string(&file)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
    {
        Ok(f) => f,
        Err(error) => {
            app::set_link(Link::Unreachable(format!("Demo file: {error}")));
            return;
        }
    };
    let now = gtk::glib::DateTime::now_utc().ok();
    let devices: HashMap<String, ha::DeviceInfo> = fixture
        .devices
        .iter()
        .map(|d| {
            (
                d.id.clone(),
                ha::DeviceInfo {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    area_name: d.area.clone(),
                    manufacturer: d.maker.clone(),
                    model: d.model.clone(),
                    entry_type: None,
                    labels: d.labels.clone(),
                },
            )
        })
        .collect();
    let mut entities = std::collections::BTreeMap::new();
    for f in fixture.entities {
        let Ok(mut e) = serde_json::from_value::<ha::Entity>(serde_json::json!({
            "entity_id": f.entity_id,
            "state": f.state,
            "attributes": f.attributes,
        })) else {
            continue;
        };
        let device = f.device.as_ref().and_then(|id| devices.get(id));
        e.device_id = f.device.clone();
        e.device_name = device.map(|d| d.name.clone());
        e.area_name = f.area.or_else(|| device.and_then(|d| d.area_name.clone()));
        e.entity_category = f.category;
        e.last_changed = now
            .as_ref()
            .and_then(|n| n.add_minutes(-(f.minutes_ago as i32)).ok())
            .and_then(|t| t.format_iso8601().ok())
            .map(|t| t.to_string());
        // An event's state is when it last fired.
        if e.domain() == "event"
            && let Some(when) = &e.last_changed
        {
            e.state = when.clone();
        }
        entities.insert(e.entity_id.clone(), e);
    }
    {
        let mut home = app.home.borrow_mut();
        home.areas = fixture
            .areas
            .iter()
            .map(|name| ha::Area {
                id: name.to_lowercase().replace(' ', "_"),
                name: name.clone(),
            })
            .collect();
        home.device_count = devices.len();
        home.devices = devices;
        home.info = ha::Info {
            version: "demo".into(),
            location_name: fixture.location_name,
        };
        home.entities = entities;
    }
    app::set_link(Link::Online);
}
