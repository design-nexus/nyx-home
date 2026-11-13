//! Home's own symbolic icons, for things the icon themes have no clear icon
//! for. Written once to the app's cache and added to the icon theme, as the
//! suite's Settings icon is. The first few are filled shapes; the rest are
//! drawn with strokes, as the mockups' icons are.

/// Automations: a trigger branching to two actions.
pub const AUTOMATIONS: &str = "nyx-home-automations-symbolic";

const AUTOMATIONS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
<g fill="#000000">
<circle cx="5" cy="12" r="2.6"/>
<circle cx="19" cy="6" r="2.6"/>
<circle cx="19" cy="18" r="2.6"/>
<rect x="7" y="11.2" width="5" height="1.6" rx="0.8"/>
<rect x="11.2" y="5.2" width="1.6" height="13.6" rx="0.8"/>
<rect x="11.2" y="5.2" width="5.6" height="1.6" rx="0.8"/>
<rect x="11.2" y="17.2" width="5.6" height="1.6" rx="0.8"/>
</g>
</svg>
"##;

/// A robot's map: a folded map of three panels.
pub const MAP: &str = "nyx-home-map-symbolic";

const MAP_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
<g fill="#000000">
<path d="M3 5.6 8.4 3.5v15L3 20.6Z"/>
<path d="M9.8 3.5l4.4 2.1v15l-4.4-2.1Z"/>
<path d="M15.6 5.6 21 3.5v15l-5.4 2.1Z"/>
</g>
</svg>
"##;

/// Play: a rounded triangle.
pub const PLAY: &str = "nyx-home-play-symbolic";

const PLAY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
<path fill="#000000" d="M8 5.2v13.6a1 1 0 0 0 1.5.9l11-6.8a1 1 0 0 0 0-1.8l-11-6.8A1 1 0 0 0 8 5.2z"/>
</svg>
"##;

pub const BULB: &str = "nyx-home-bulb-symbolic";
pub const THERMO: &str = "nyx-home-thermo-symbolic";
pub const FAN: &str = "nyx-home-fan-symbolic";
pub const SPEAKER: &str = "nyx-home-speaker-symbolic";
pub const PLUG: &str = "nyx-home-plug-symbolic";
pub const LOCK: &str = "nyx-home-lock-symbolic";
pub const BLINDS: &str = "nyx-home-blinds-symbolic";
pub const ROBOT: &str = "nyx-home-robot-symbolic";
pub const MOWER: &str = "nyx-home-mower-symbolic";
pub const CAMERA: &str = "nyx-home-camera-symbolic";
pub const SHIELD: &str = "nyx-home-shield-symbolic";
pub const PEN: &str = "nyx-home-pen-symbolic";
pub const EXPAND: &str = "nyx-home-expand-symbolic";
pub const GRID: &str = "nyx-home-grid-symbolic";
pub const POWER: &str = "nyx-home-power-symbolic";
pub const SCENE: &str = "nyx-home-scene-symbolic";
pub const PIN: &str = "nyx-home-pin-symbolic";

const SOFA: &str = "nyx-home-sofa-symbolic";
const UTENSILS: &str = "nyx-home-utensils-symbolic";
const BED: &str = "nyx-home-bed-symbolic";
const MONITOR: &str = "nyx-home-monitor-symbolic";
const DOOR: &str = "nyx-home-door-symbolic";
const GARAGE: &str = "nyx-home-garage-symbolic";
const TREE: &str = "nyx-home-tree-symbolic";
const DROP: &str = "nyx-home-drop-symbolic";
const HOUSE: &str = "nyx-home-house-symbolic";

/// Stroked icons: `(name, the shapes inside the 24 × 24 frame)`.
const STROKED: &[(&str, &str)] = &[
    (
        BULB,
        r#"<path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V16h5v-.1c0-.8.4-1.5 1-2A6 6 0 0 0 12 3z"/>"#,
    ),
    (
        THERMO,
        r#"<path d="M14 14.8V5a2 2 0 0 0-4 0v9.8a4 4 0 1 0 4 0z"/><path d="M12 11v6"/>"#,
    ),
    (
        FAN,
        r#"<circle cx="12" cy="12" r="1.8"/><path d="M12 10.2C11 6 12.5 3 15 3.5s2 4.5-3 6.7M13.8 12c4.2-1 7.2.5 6.7 3s-4.5 2-6.7-3M12 13.8c1 4.2-.5 7.2-3 6.7s-2-4.5 3-6.7M10.2 12C6 13 3 11.5 3.5 9s4.5-2 6.7 3"/>"#,
    ),
    (
        SPEAKER,
        r#"<rect x="5" y="3" width="14" height="18" rx="3"/><circle cx="12" cy="14" r="3.5"/><path d="M12 7h.01" stroke-width="2.6"/>"#,
    ),
    (
        PLUG,
        r#"<path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4"/>"#,
    ),
    (
        LOCK,
        r#"<rect x="4.5" y="11" width="15" height="10" rx="2.5"/><path d="M8 11V7.5a4 4 0 0 1 8 0V11"/>"#,
    ),
    (
        BLINDS,
        r#"<path d="M4 3.5h16M5 3.5v10h14v-10M5 7h14M5 10.5h14M12 13.5v4"/><circle cx="12" cy="19" r="1.3"/>"#,
    ),
    (
        ROBOT,
        r#"<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="9" r="2.5"/><path d="M6.5 15.5h11"/>"#,
    ),
    (
        MOWER,
        r#"<path d="M3 16h14l2.5-6H10L8 6H4"/><circle cx="7" cy="17.5" r="2"/><circle cx="16" cy="17.5" r="2"/>"#,
    ),
    (
        CAMERA,
        r#"<rect x="3" y="7" width="13" height="10" rx="2.5"/><path d="m16 11 5-3v8l-5-3"/>"#,
    ),
    (
        SHIELD,
        r#"<path d="M12 21s7.5-3.6 7.5-9.5V5.5L12 3 4.5 5.5v6C4.5 17.4 12 21 12 21z"/>"#,
    ),
    (
        PEN,
        r#"<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>"#,
    ),
    (
        EXPAND,
        r#"<path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15"/>"#,
    ),
    (
        GRID,
        r#"<rect x="4" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5"/>"#,
    ),
    (POWER, r#"<path d="M12 3v9M17.7 6.6a8 8 0 1 1-11.4 0"/>"#),
    (
        SCENE,
        r#"<path d="M12 3.5l2.2 5.3 5.3 2.2-5.3 2.2L12 18.5l-2.2-5.3L4.5 11l5.3-2.2z"/>"#,
    ),
    (
        PIN,
        r#"<path d="M12 21s7-6.2 7-11.5a7 7 0 0 0-14 0C5 14.8 12 21 12 21z"/><circle cx="12" cy="9.5" r="2.5"/>"#,
    ),
    (
        SOFA,
        r#"<path d="M5 11V8a2.5 2.5 0 0 1 2.5-2.5h9A2.5 2.5 0 0 1 19 8v3"/><path d="M3 13a2 2 0 0 1 4 0v1.5h10V13a2 2 0 0 1 4 0v4.5H3zM5 17.5V20M19 17.5V20"/>"#,
    ),
    (
        UTENSILS,
        r#"<path d="M7 3v8M5 3v5a2 2 0 0 0 4 0V3M7 11v10M17 21V3c-2 1.5-3 4-3 7h3"/>"#,
    ),
    (
        BED,
        r#"<path d="M3 19V6M3 14h18v5M21 14v-2.5A2.5 2.5 0 0 0 18.5 9H11v5"/><circle cx="6.8" cy="11.3" r="1.6"/>"#,
    ),
    (
        MONITOR,
        r#"<rect x="3" y="4" width="18" height="12" rx="2.5"/><path d="M8.5 20h7M12 16v4"/>"#,
    ),
    (
        DOOR,
        r#"<path d="M6 21V4.5A1.5 1.5 0 0 1 7.5 3h9A1.5 1.5 0 0 1 18 4.5V21M4 21h16"/><path d="M14.5 12h.01" stroke-width="2.6"/>"#,
    ),
    (
        GARAGE,
        r#"<path d="M3 10 12 4l9 6v10.5H3z"/><path d="M7 20.5V13h10v7.5M7 16.5h10"/>"#,
    ),
    (TREE, r#"<path d="M12 21v-5M12 3 6 11h3l-4 5h14l-4-5h3z"/>"#),
    (
        DROP,
        r#"<path d="M12 3.5s6 6.5 6 11a6 6 0 0 1-12 0c0-4.5 6-11 6-11z"/>"#,
    ),
    (
        HOUSE,
        r#"<path d="M4 10.5 12 4l8 6.5V19a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 19z"/><path d="M9.5 20.5v-6h5v6"/>"#,
    ),
];

/// GTK fills every shape of a symbolic icon unless the shape is marked as an
/// outline in the foreground colour.
fn stroked(body: &str) -> String {
    let mut body = body.to_string();
    for shape in ["path", "circle", "rect"] {
        body = body.replace(
            &format!("<{shape} "),
            &format!("<{shape} fill=\"none\" class=\"transparent-fill foreground-stroke\" "),
        );
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
<g fill="none" stroke="#000000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
{body}
</g>
</svg>
"##
    )
}

/// A room's icon and swatch, from words in its name.
pub fn room(name: &str) -> (&'static str, &'static str) {
    let name = name.to_lowercase();
    let words: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).collect();
    let has = |starts: &[&str]| {
        words
            .iter()
            .any(|w| starts.iter().any(|s| w.starts_with(s)))
    };
    if has(&["living", "lounge", "family", "den"]) {
        (SOFA, "violet")
    } else if has(&["kitchen", "dining"]) {
        (UTENSILS, "coral")
    } else if has(&["bed", "nursery", "guest"]) {
        (BED, "blue")
    } else if has(&["office", "study", "studio", "desk"]) {
        (MONITOR, "amber")
    } else if has(&["hall", "entr", "porch", "stair", "landing"]) {
        (DOOR, "red")
    } else if has(&[
        "garage", "shed", "workshop", "basement", "utility", "laundry",
    ]) {
        (GARAGE, "graphite")
    } else if has(&[
        "garden", "yard", "patio", "outside", "outdoor", "balcony", "terrace",
    ]) {
        (TREE, "green")
    } else if has(&["bath", "toilet", "shower", "wc"]) {
        (DROP, "teal")
    } else {
        (HOUSE, "pink")
    }
}

pub fn install() {
    let dir = nyx_core::paths::cache_dir().join("icons");
    let mut all: Vec<(&str, String)> = vec![
        (AUTOMATIONS, AUTOMATIONS_SVG.to_string()),
        (MAP, MAP_SVG.to_string()),
        (PLAY, PLAY_SVG.to_string()),
    ];
    all.extend(STROKED.iter().map(|(name, body)| (*name, stroked(body))));
    for (name, svg) in all {
        let path = dir.join(format!("{name}.svg"));
        if std::fs::read_to_string(&path).ok().as_deref() != Some(svg.as_str()) {
            let _ = nyx_core::cmd::atomic_write(&path, &svg);
        }
    }
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(dir);
    }
}
