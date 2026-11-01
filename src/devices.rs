//! The devices other Nyx apps show (the Control Center's Home tile): lights, switches,
//! fans, climate and scenes, by room, served by `nyx-homed` as
//! `io.github.design_nexus.nyx.Home.Devices1` so nothing has to open Home to see them.
//!
//! `Snapshot() -> s` returns [`Snapshot`] as JSON; `Call(s entity, s service, s data)`
//! runs one of the few actions in [`allowed`]. Everything else Home Assistant can do
//! stays in Home.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub const PATH: &str = "/io/github/design_nexus/nyx/Home/Devices";
pub const INTERFACE: &str = "io.github.design_nexus.nyx.Home.Devices1";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub on: bool,
    pub available: bool,
    pub room: Option<String>,
    /// Lights: 0.0–1.0 while on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f64>,
    /// Lights: the colour shown, when the light has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    pub dimmable: bool,
    pub colorable: bool,
    /// Climate: the target and current temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub room: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Snapshot {
    /// Rooms that have something in them, in Home Assistant's order.
    pub rooms: Vec<String>,
    pub devices: Vec<Device>,
    pub scenes: Vec<Scene>,
}

const DOMAINS: [&str; 5] = ["light", "switch", "fan", "climate", "scene"];

/// Where each entity is: its own room, else its device's, by entity id.
pub fn rooms(
    entity_registry: &Value,
    device_registry: &Value,
    area_registry: &Value,
) -> (HashMap<String, String>, Vec<String>, Vec<String>) {
    let areas: HashMap<&str, &str> = area_registry
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| Some((a["area_id"].as_str()?, a["name"].as_str()?)))
        .collect();
    let order: Vec<String> = area_registry
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["name"].as_str().map(str::to_string))
        .collect();
    let device_area: HashMap<&str, &str> = device_registry
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["id"].as_str()?, d["area_id"].as_str()?)))
        .collect();
    let mut room = HashMap::new();
    let mut hidden = Vec::new();
    for e in entity_registry.as_array().into_iter().flatten() {
        let Some(id) = e["entity_id"].as_str() else {
            continue;
        };
        if !e["hidden_by"].is_null()
            || !e["disabled_by"].is_null()
            || !e["entity_category"].is_null()
        {
            hidden.push(id.to_string());
            continue;
        }
        let area = e["area_id"].as_str().or_else(|| {
            e["device_id"]
                .as_str()
                .and_then(|d| device_area.get(d).copied())
        });
        if let Some(name) = area.and_then(|a| areas.get(a)) {
            room.insert(id.to_string(), name.to_string());
        }
    }
    (room, hidden, order)
}

/// Build the snapshot from `/api/states` and the room lookup.
pub fn snapshot(
    states: &[Value],
    room: &HashMap<String, String>,
    hidden: &[String],
    order: &[String],
) -> Snapshot {
    let mut snap = Snapshot::default();
    for s in states {
        let Some(id) = s["entity_id"].as_str() else {
            continue;
        };
        let domain = id.split_once('.').map_or("", |d| d.0);
        if !DOMAINS.contains(&domain) || hidden.iter().any(|h| h == id) {
            continue;
        }
        let a = &s["attributes"];
        // A light group repeats its lights when they're shown too; when they're hidden
        // (the usual way to set one up), the group is the light.
        if domain == "light"
            && let Some(members) = a["entity_id"].as_array().filter(|m| !m.is_empty())
            && members
                .iter()
                .filter_map(Value::as_str)
                .any(|m| !hidden.iter().any(|h| h == m))
        {
            continue;
        }
        let name = a["friendly_name"].as_str().unwrap_or(id).to_string();
        let r = room.get(id).cloned();
        if domain == "scene" {
            snap.scenes.push(Scene {
                id: id.into(),
                name,
                room: r,
            });
            continue;
        }
        let state = s["state"].as_str().unwrap_or("");
        let modes: Vec<&str> = a["supported_color_modes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let color = a["rgb_color"].as_array().and_then(|c| {
            let v: Vec<u8> = c
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n.min(255) as u8))
                .collect();
            (v.len() == 3).then(|| [v[0], v[1], v[2]])
        });
        snap.devices.push(Device {
            id: id.into(),
            name,
            domain: domain.into(),
            on: if domain == "climate" {
                state != "off" && !state.is_empty()
            } else {
                state == "on"
            },
            available: state != "unavailable" && state != "unknown",
            room: r,
            brightness: a["brightness"]
                .as_f64()
                .map(|b| (b / 255.0).clamp(0.0, 1.0)),
            color,
            dimmable: domain == "light" && modes.iter().any(|m| *m != "onoff"),
            colorable: modes
                .iter()
                .any(|m| matches!(*m, "hs" | "rgb" | "rgbw" | "rgbww" | "xy")),
            target: a["temperature"].as_f64(),
            current: a["current_temperature"].as_f64(),
        });
    }
    let used: Vec<&String> = snap
        .devices
        .iter()
        .filter_map(|d| d.room.as_ref())
        .chain(snap.scenes.iter().filter_map(|s| s.room.as_ref()))
        .collect();
    snap.rooms = order.iter().filter(|r| used.contains(r)).cloned().collect();
    snap
}

/// The actions other apps may run through `Call`. Anything else is refused before
/// Home Assistant is asked.
pub fn allowed(entity_id: &str, service: &str, data: &Value) -> bool {
    let domain = entity_id.split_once('.').map_or("", |d| d.0);
    let keys_ok = |keys: &[&str]| {
        data.as_object()
            .is_none_or(|m| m.keys().all(|k| keys.contains(&k.as_str())))
    };
    match (domain, service) {
        ("light", "turn_on") => keys_ok(&["brightness_pct", "rgb_color"]),
        ("light" | "switch" | "fan", "turn_off")
        | ("switch" | "fan", "turn_on")
        | ("scene", "turn_on") => keys_ok(&[]),
        ("climate", "set_temperature") => keys_ok(&["temperature"]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registries() -> (Value, Value, Value) {
        (
            json!([
                {"entity_id": "light.ceiling", "device_id": "d1", "area_id": null, "hidden_by": null, "disabled_by": null, "entity_category": null},
                {"entity_id": "light.lamp", "device_id": "d2", "area_id": "bedroom", "hidden_by": null, "disabled_by": null, "entity_category": null},
                {"entity_id": "switch.identify", "device_id": "d1", "area_id": null, "hidden_by": null, "disabled_by": null, "entity_category": "config"},
                {"entity_id": "scene.evening", "area_id": "living", "hidden_by": null, "disabled_by": null, "entity_category": null}
            ]),
            json!([{"id": "d1", "area_id": "living"}, {"id": "d2", "area_id": "living"}]),
            json!([{"area_id": "living", "name": "Living room"}, {"area_id": "bedroom", "name": "Bedroom"}, {"area_id": "garage", "name": "Garage"}]),
        )
    }

    #[test]
    fn rooms_come_from_the_entity_then_its_device() {
        let (e, d, a) = registries();
        let (room, hidden, order) = rooms(&e, &d, &a);
        assert_eq!(room["light.ceiling"], "Living room", "from its device");
        assert_eq!(room["light.lamp"], "Bedroom", "its own room wins");
        assert_eq!(hidden, vec!["switch.identify"]);
        assert_eq!(order, vec!["Living room", "Bedroom", "Garage"]);
    }

    #[test]
    fn snapshot_keeps_what_the_tile_shows() {
        let (e, d, a) = registries();
        let (room, hidden, order) = rooms(&e, &d, &a);
        let states = vec![
            json!({"entity_id": "light.ceiling", "state": "on", "attributes": {"friendly_name": "Ceiling", "brightness": 178, "supported_color_modes": ["brightness"]}}),
            json!({"entity_id": "light.lamp", "state": "on", "attributes": {"friendly_name": "Lamp", "rgb_color": [157, 123, 255], "supported_color_modes": ["hs"]}}),
            json!({"entity_id": "light.all", "state": "on", "attributes": {"entity_id": ["light.ceiling", "light.lamp"]}}),
            json!({"entity_id": "switch.identify", "state": "off", "attributes": {}}),
            json!({"entity_id": "climate.heat", "state": "heat", "attributes": {"friendly_name": "Heating", "temperature": 21.5, "current_temperature": 20.0}}),
            json!({"entity_id": "scene.evening", "state": "scening", "attributes": {"friendly_name": "Evening"}}),
            json!({"entity_id": "sensor.power", "state": "40", "attributes": {}}),
        ];
        let s = snapshot(&states, &room, &hidden, &order);
        let ids: Vec<&str> = s.devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["light.ceiling", "light.lamp", "climate.heat"],
            "no groups, config entities or sensors"
        );
        assert!((s.devices[0].brightness.unwrap() - 0.698).abs() < 0.01);
        assert!(s.devices[0].dimmable && !s.devices[0].colorable);
        assert_eq!(s.devices[1].color, Some([157, 123, 255]));
        assert!(s.devices[2].on);
        assert_eq!(s.devices[2].target, Some(21.5));
        assert_eq!(s.scenes[0].room.as_deref(), Some("Living room"));
        assert_eq!(
            s.rooms,
            vec!["Living room", "Bedroom"],
            "only rooms with something in them"
        );
    }

    #[test]
    fn a_group_of_hidden_lights_is_the_light() {
        let hidden = vec!["light.a".to_string(), "light.b".to_string()];
        let states = vec![
            json!({"entity_id": "light.a", "state": "on", "attributes": {}}),
            json!({"entity_id": "light.kitchen", "state": "on", "attributes": {"friendly_name": "Kitchen", "entity_id": ["light.a", "light.b"]}}),
            json!({"entity_id": "light.mixed", "state": "on", "attributes": {"entity_id": ["light.a", "light.c"]}}),
            json!({"entity_id": "light.c", "state": "off", "attributes": {}}),
        ];
        let s = snapshot(&states, &HashMap::new(), &hidden, &[]);
        let ids: Vec<&str> = s.devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["light.kitchen", "light.c"],
            "the group of hidden lights, not the mixed one"
        );
    }

    #[test]
    fn only_the_few_actions_are_allowed() {
        assert!(allowed(
            "light.lamp",
            "turn_on",
            &json!({"brightness_pct": 40})
        ));
        assert!(allowed(
            "light.lamp",
            "turn_on",
            &json!({"rgb_color": [1, 2, 3]})
        ));
        assert!(!allowed("light.lamp", "turn_on", &json!({"flash": "long"})));
        assert!(allowed("scene.evening", "turn_on", &json!({})));
        assert!(allowed(
            "climate.heat",
            "set_temperature",
            &json!({"temperature": 21})
        ));
        assert!(!allowed("lock.front", "unlock", &json!({})));
        assert!(!allowed("light.lamp", "toggle", &Value::Null));
    }
}
