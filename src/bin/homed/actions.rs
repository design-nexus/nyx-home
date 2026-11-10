//! Home on the control interface (API 2, nyx-actions' server), beside the daemon's own
//! interfaces: devices, lights, scenes, climate and routines for the `nyx` command,
//! Glimpse and Nyx Assistant, and the `home` topic Glimpse's Home panel shows. Calls go
//! to Home Assistant through the same allowlist as `Home.Devices1`.

use super::{Devices, Engine, Start};
use nyx_actions::server::{self, Handler, Request, Server};
use nyx_actions::{Error, ErrorKind};
use nyx_home::automation::{BUS, Owner, PATH};
use nyx_home::devices::{Device, Snapshot};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const APP: &str = "io.github.design_nexus.nyx.Home";

/// The served interface, for `Home.Devices1` to check its callers the same way.
pub static SERVER: OnceLock<Server> = OnceLock::new();

/// While watched, the topic is read again this often.
const REFRESH: Duration = Duration::from_secs(3);

pub fn description() -> Value {
    let device = json!({"type": "string", "description": "a device's name or ID"});
    let switch_alias = |word: &str, on: Option<bool>| match on {
        Some(on) => json!({"word": word, "arguments": {"on": on}}),
        None => json!(word),
    };
    json!({
        "tools": [
            {"name": "home.devices", "description": "Home's devices: lights, switches, fans and climate, by room, on or off.",
             "inputSchema": {"type": "object", "properties": {"room": {"type": "string"}}, "additionalProperties": false},
             "annotations": {"title": "Devices"}, "x-nyx": {"risk": "read_only", "positional": ["room"], "aliases": ["list"], "result": "list"}},
            {"name": "home.switch", "description": "Turn a light, switch or fan on or off (or toggle it).",
             "inputSchema": {"type": "object", "properties": {"device": device, "on": {"type": "boolean"}}, "required": ["device"], "additionalProperties": false},
             "annotations": {"title": "Switch"},
             "x-nyx": {"risk": "writes", "positional": ["device"], "result": "message",
                       "aliases": [switch_alias("on", Some(true)), switch_alias("off", Some(false)), switch_alias("toggle", None)]}},
            {"name": "home.light", "description": "Set a light's brightness (0 to 100) and, if it has one, its colour (#rrggbb).",
             "inputSchema": {"type": "object", "properties": {"device": device, "brightness": {"type": "integer", "minimum": 1, "maximum": 100}, "color": {"type": "string", "pattern": "^#[0-9a-fA-F]{6}$"}},
                             "required": ["device"], "additionalProperties": false},
             "annotations": {"title": "Light"}, "x-nyx": {"risk": "writes", "positional": ["device", "brightness"], "aliases": ["dim"], "result": "message"}},
            {"name": "home.scene", "description": "Turn on a scene.",
             "inputSchema": {"type": "object", "properties": {"scene": {"type": "string"}}, "required": ["scene"], "additionalProperties": false},
             "annotations": {"title": "Scene"}, "x-nyx": {"risk": "writes", "positional": ["scene"], "result": "message"}},
            {"name": "home.climate", "description": "Set a thermostat's target temperature.",
             "inputSchema": {"type": "object", "properties": {"device": device, "temperature": {"type": "number"}}, "required": ["device", "temperature"], "additionalProperties": false},
             "annotations": {"title": "Climate"}, "x-nyx": {"risk": "writes", "positional": ["device", "temperature"], "aliases": ["heat", "temperature"], "result": "message"}},
            {"name": "home.routines", "description": "The routines Home can run (its desktop automations).",
             "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
             "annotations": {"title": "Routines"}, "x-nyx": {"risk": "read_only", "result": "list"}},
            {"name": "home.run", "description": "Run one of Home's routines now. A routine can do anything it was built to, so this asks first.",
             "inputSchema": {"type": "object", "properties": {"routine": {"type": "string"}}, "required": ["routine"], "additionalProperties": false},
             "annotations": {"title": "Run a routine"}, "x-nyx": {"risk": "writes", "confirmation": "ask", "positional": ["routine"], "result": "message"}},
            {"name": "home.open", "description": "Open Home's window.",
             "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
             "annotations": {"title": "Open Home"}, "x-nyx": {"risk": "writes", "confirmation": "never", "result": "none"}}
        ],
        "nyx": {
            "api": 2, "app": APP, "name": "Home", "icon": APP, "namespaces": ["home"], "aliases": ["lights"],
            "bus_name": BUS, "object_path": PATH, "activation": "dbus", "default": "home.devices", "open": {"action": "home.open"},
            "topics": [{"name": "home"}],
            "panel": {"id": "home", "topics": ["home"], "size": {"min": [440, 340], "preferred": [520, 420]}}
        }
    })
}

/// Serve Home's actions on the daemon's connection, and keep the topic current while
/// something watches it.
pub async fn serve(conn: &zbus::Connection, engine: Engine) -> Result<(), String> {
    let devices = Arc::new(Devices::default());
    let own = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf()));
    let (d, e) = (devices.clone(), engine.clone());
    let handler: Handler = Arc::new(move |r: Request| {
        let (d, e) = (d.clone(), e.clone());
        Box::pin(async move {
            let result = handle(&d, &e, r).await;
            if let Some(s) = SERVER.get()
                && s.watched("home")
            {
                s.publish("home", topic(&d, &e).await).await;
            }
            result
        })
    });
    let served = server::serve(conn, PATH, BUS, description(), own, handler).await?;
    let task: Arc<Mutex<Option<tokio::task::AbortHandle>>> = Arc::default();
    served.on_watch(move |name, on| {
        if name != "home" {
            return;
        }
        let mut task = task.lock().unwrap();
        if let Some(t) = task.take() {
            t.abort();
        }
        if on {
            let (d, e) = (devices.clone(), engine.clone());
            *task = Some(
                tokio::spawn(async move {
                    loop {
                        if let Some(s) = SERVER.get() {
                            s.publish("home", topic(&d, &e).await).await;
                        }
                        tokio::time::sleep(REFRESH).await;
                    }
                })
                .abort_handle(),
            );
        }
    });
    let _ = SERVER.set(served);
    Ok(())
}

/// What the panel shows: rooms, devices (favorites marked), scenes and routines.
async fn topic(d: &Devices, e: &Engine) -> Value {
    match snapshot(d).await {
        Ok(snap) => {
            let favorites = favorites(d).await;
            let devices: Vec<Value> = snap
                .devices
                .iter()
                .map(|dev| {
                    let mut v = serde_json::to_value(dev).unwrap_or_default();
                    v["favorite"] = json!(favorites.contains(&dev.id));
                    v
                })
                .collect();
            json!({"rooms": snap.rooms, "devices": devices, "scenes": snap.scenes, "routines": routines(e), "error": null})
        }
        Err(e) => json!({"error": e.message}),
    }
}

async fn snapshot(d: &Devices) -> Result<Snapshot, Error> {
    let unreachable = |e: String| Error::new(ErrorKind::Unavailable, format!("Home Assistant isn't reachable: {e}"));
    let client = d.client().await.map_err(unreachable)?;
    let states = match client.states().await {
        Ok(s) => s,
        Err(e) => {
            d.forget_client().await;
            return Err(unreachable(e));
        }
    };
    let (room, hidden, order) = d.room_cache(&client).await.unwrap_or_default();
    Ok(nyx_home::devices::snapshot(&states, &room, &hidden, &order))
}

/// Entities on the devices pinned to Home's Overview (its `favorite_devices`).
async fn favorites(d: &Devices) -> Vec<String> {
    static CACHE: Mutex<Option<(Instant, std::collections::HashMap<String, String>)>> = Mutex::new(None);
    let pinned: Vec<String> = std::fs::read_to_string(nyx_core::paths::app_dir().join("settings.toml"))
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .and_then(|t| t.get("favorite_devices").and_then(|v| v.as_array()).cloned())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    if pinned.is_empty() {
        return Vec::new();
    }
    let fresh = CACHE.lock().unwrap().as_ref().is_some_and(|(at, _)| at.elapsed() < Duration::from_secs(600));
    if !fresh
        && let Ok(client) = d.client().await
        && let Ok(lists) = client.ws_lists(&["config/entity_registry/list"]).await
    {
        let map = lists[0]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| Some((e["entity_id"].as_str()?.to_string(), e["device_id"].as_str()?.to_string())))
            .collect();
        *CACHE.lock().unwrap() = Some((Instant::now(), map));
    }
    let cache = CACHE.lock().unwrap();
    let Some((_, map)) = cache.as_ref() else { return Vec::new() };
    map.iter().filter(|(_, device)| pinned.contains(device)).map(|(entity, _)| entity.clone()).collect()
}

fn routines(e: &Engine) -> Vec<Value> {
    e.list()
        .map(|list| list.iter().filter(|w| w.enabled && w.owner != Owner::HomeAssistant).map(|w| json!({"id": w.id, "name": w.name})).collect())
        .unwrap_or_default()
}

/// One of `items` by its ID, or its name (exactly, then the one whose name has the words).
fn pick<'a, T>(items: &'a [T], word: &str, id: impl Fn(&T) -> &str, name: impl Fn(&T) -> &str, what: &str) -> Result<&'a T, Error> {
    let w = word.trim().to_lowercase();
    if let Some(i) = items.iter().find(|i| id(i) == word || name(i).to_lowercase() == w) {
        return Ok(i);
    }
    let found: Vec<&T> = items.iter().filter(|i| name(i).to_lowercase().contains(&w)).collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(Error::new(ErrorKind::InvalidArguments, format!("no {what} is called “{word}”"))),
        many => Err(Error::new(
            ErrorKind::InvalidArguments,
            format!("“{word}” could be {}", many.iter().take(5).map(|i| name(i).to_string()).collect::<Vec<_>>().join(", ")),
        )),
    }
}

async fn call(d: &Devices, entity: &str, service: &str, data: Value) -> Result<(), Error> {
    if !nyx_home::devices::allowed(entity, service, &data) {
        return Err(Error::new(ErrorKind::PermissionDenied, format!("{service} on {entity} isn't offered here")));
    }
    let domain = entity.split_once('.').map_or("", |p| p.0);
    let client = d.client().await.map_err(|e| Error::new(ErrorKind::Unavailable, format!("Home Assistant isn't reachable: {e}")))?;
    client.call_with(domain, service, entity, data).await.map_err(|e| Error::new(ErrorKind::Failed, e))
}

fn text<'a>(args: &'a Value, key: &str) -> &'a str {
    args[key].as_str().unwrap_or("")
}

async fn handle(d: &Devices, e: &Engine, r: Request) -> Result<Value, Error> {
    let args = &r.arguments;
    let done = |m: String| Ok(json!({"message": m}));
    match r.action.as_str() {
        "home.devices" => {
            let snap = snapshot(d).await?;
            let room = text(args, "room").to_lowercase();
            let items: Vec<Value> = snap
                .devices
                .iter()
                .filter(|dev| room.is_empty() || dev.room.as_deref().is_some_and(|r| r.to_lowercase() == room))
                .map(|dev| json!({"title": dev.name, "subtitle": state_line(dev), "id": dev.id}))
                .collect();
            Ok(json!({"items": items}))
        }
        "home.switch" => {
            let snap = snapshot(d).await?;
            let switchable: Vec<Device> = snap.devices.into_iter().filter(|x| matches!(x.domain.as_str(), "light" | "switch" | "fan")).collect();
            let dev = pick(&switchable, text(args, "device"), |x| &x.id, |x| &x.name, "light, switch or fan")?;
            let on = args["on"].as_bool().unwrap_or(!dev.on);
            call(d, &dev.id, if on { "turn_on" } else { "turn_off" }, json!({})).await?;
            done(format!("{} {}", dev.name, if on { "on" } else { "off" }))
        }
        "home.light" => {
            let snap = snapshot(d).await?;
            let lights: Vec<Device> = snap.devices.into_iter().filter(|x| x.domain == "light").collect();
            let dev = pick(&lights, text(args, "device"), |x| &x.id, |x| &x.name, "light")?;
            let mut data = serde_json::Map::new();
            if let Some(b) = args["brightness"].as_i64() {
                data.insert("brightness_pct".into(), json!(b));
            }
            if let Some(hex) = args["color"].as_str() {
                let c = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
                data.insert("rgb_color".into(), json!([c(1), c(3), c(5)]));
            }
            call(d, &dev.id, "turn_on", Value::Object(data)).await?;
            done(format!("{} set", dev.name))
        }
        "home.scene" => {
            let snap = snapshot(d).await?;
            let scene = pick(&snap.scenes, text(args, "scene"), |x| &x.id, |x| &x.name, "scene")?;
            call(d, &scene.id, "turn_on", json!({})).await?;
            done(format!("{} on", scene.name))
        }
        "home.climate" => {
            let snap = snapshot(d).await?;
            let climate: Vec<Device> = snap.devices.into_iter().filter(|x| x.domain == "climate").collect();
            let dev = pick(&climate, text(args, "device"), |x| &x.id, |x| &x.name, "thermostat")?;
            let t = args["temperature"].as_f64().unwrap_or_default();
            call(d, &dev.id, "set_temperature", json!({"temperature": t})).await?;
            done(format!("{} set to {t}°", dev.name))
        }
        "home.routines" => {
            let items: Vec<Value> = routines(e).into_iter().map(|w| json!({"title": w["name"], "id": w["id"]})).collect();
            Ok(json!({"items": items}))
        }
        "home.run" => {
            let list: Vec<Value> = routines(e);
            let w = pick(&list, text(args, "routine"), |x| x["id"].as_str().unwrap_or(""), |x| x["name"].as_str().unwrap_or(""), "routine")?;
            let id = w["id"].as_str().unwrap_or_default();
            let workflow = e.get(id).ok().flatten().ok_or_else(|| Error::new(ErrorKind::Failed, "that routine is gone"))?;
            e.run(workflow, "nyx".into(), Start::Manual, None).map_err(|err| Error::new(ErrorKind::Failed, err.to_string()))?;
            done(format!("Running {}", w["name"].as_str().unwrap_or("it")))
        }
        "home.open" => {
            let program = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("nyx-home"))).unwrap_or_else(|| "nyx-home".into());
            std::process::Command::new(program).spawn().map_err(|err| Error::new(ErrorKind::Failed, format!("Home didn't open: {err}")))?;
            Ok(Value::Null)
        }
        other => Err(Error::new(ErrorKind::UnknownAction, format!("{other} isn't an action here"))),
    }
}

fn state_line(dev: &Device) -> String {
    let mut s = if dev.available { if dev.on { "On".to_string() } else { "Off".to_string() } } else { "Unavailable".to_string() };
    if let Some(b) = dev.brightness.filter(|_| dev.on) {
        s.push_str(&format!(" · {}%", (b * 100.0).round()));
    }
    if let Some(room) = &dev.room {
        s.push_str(&format!(" · {room}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_description_is_api_2() {
        let d = nyx_actions::Description::from_value(&description(), None).unwrap();
        assert_eq!(d.app.bus_name, BUS);
        assert_eq!(d.app.panel.as_ref().map(|p| p.id.as_str()), Some("home"));
        assert!(d.action("home.run").unwrap().asks());
        assert!(!d.action("home.switch").unwrap().asks());
    }
}
