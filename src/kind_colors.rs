//! One color per kind of device, taken from the active theme's named swatches
//! so light, dark and Omarchy themes each get their own. Card backgrounds stay
//! neutral; the color is on the icon chip, the glow of an active card, sliders,
//! chosen segments and indicators.

/// Card class and the swatch it uses.
const KINDS: &[(&str, &str)] = &[
    ("kind-light", "amber"),
    ("kind-climate", "coral"),
    ("kind-fan", "blue"),
    ("kind-media", "pink"),
    ("kind-camera", "teal"),
    ("kind-robot", "violet"),
    ("kind-security", "red"),
    ("kind-cover", "blue"),
    ("kind-switch", "violet"),
    ("kind-sensor", "green"),
];

/// Swatches for scene tiles, in the order their hash picks them.
const SCENES: [&str; 8] = [
    "blue", "violet", "pink", "red", "coral", "amber", "green", "teal",
];

/// Swatches rooms take, by the words in their names.
const ROOMS: [&str; 9] = [
    "violet", "coral", "blue", "amber", "red", "graphite", "green", "teal", "pink",
];

/// Defines `@kind_*` colors and the rules that use them. `swatches` are the
/// theme's `(name, color)` pairs; a missing name falls back to `accent`.
pub fn css(swatches: &[(String, String)], accent: &str) -> String {
    let swatch = |name: &str| {
        swatches
            .iter()
            .find(|(n, _)| n == name)
            .map_or(accent, |(_, c)| c.as_str())
            .to_string()
    };
    let mut css = String::new();
    for (class, name) in KINDS {
        let color = swatch(name);
        let var = class.replace('-', "_");
        css.push_str(&format!("@define-color {var} {color};\n"));
        css.push_str(&rules(&format!(".{class}"), &format!("@{var}")));
    }
    // Scene tiles and chips each take one of these, by a hash of their ID.
    for (i, name) in SCENES.iter().enumerate() {
        let color = swatch(name);
        for scene in ["scene-tile", "scene-chip"] {
            css.push_str(&format!(
                "window.home-window .{scene}.scene-color-{i} {{ background: alpha({color}, 0.16); box-shadow: inset 0 0 0 1px alpha({color}, 0.3); }}
window.home-window .{scene}.scene-color-{i}:hover {{ background: alpha({color}, 0.26); }}
window.home-window .{scene}.scene-color-{i} image {{ color: {color}; }}
"
            ));
        }
    }
    // A camera with no picture yet: two of the scene colors.
    for i in 0..SCENES.len() {
        let (a, b) = (swatch(SCENES[i]), swatch(SCENES[(i + 3) % SCENES.len()]));
        css.push_str(&format!(
            "window.home-window .camera-blank.scene-color-{i} {{ background: linear-gradient(135deg, alpha({a}, 0.7), alpha({b}, 0.7)); }}\n"
        ));
    }
    // Each room keeps a swatch (icons::room) for its icon.
    for name in ROOMS {
        let color = swatch(name);
        css.push_str(&format!(
            "window.home-window .room-{name} image.room-chip {{ color: {color}; background: alpha({color}, 0.18); }}
window.home-window .room-{name} image.room-chip.big {{ color: @theme_bg; background: {color}; }}
"
        ));
    }
    // A thermostat takes the color of what it's doing.
    for (state, name) in [("heating", "coral"), ("cooling", "blue")] {
        let color = swatch(name);
        css.push_str(&format!("@define-color kind_{state} {color};\n"));
        css.push_str(&rules(
            &format!(".kind-climate.{state}"),
            &format!("@kind_{state}"),
        ));
        css.push_str(&format!(
            "window.home-window .climate-action.{state}, window.home-window .{state} .big-readout {{ color: @kind_{state}; }}\n"
        ));
    }
    css
}

fn rules(scope: &str, c: &str) -> String {
    format!(
        "window.home-window {scope} image.chip {{ color: {c}; background: alpha({c}, 0.16); }}
window.home-window .device-card{scope}.on image.chip {{ color: @theme_bg; background: {c}; }}
window.home-window .drawer-row{scope}.on image.chip, window.home-window .robot-row{scope}.on image.chip {{ color: @theme_bg; background: {c}; }}
window.home-window .device-card{scope}.on {{ box-shadow: inset 0 0 0 1px alpha({c}, 0.45), 0 8px 24px -10px alpha({c}, 0.6); }}
window.home-window .device-card{scope} scale trough highlight {{ background: {c}; }}
window.home-window .device-card{scope} scale slider {{ background: {c}; }}
window.home-window .device-card{scope} switch:checked {{ background: {c}; }}
window.home-window .device-card{scope} .segment:checked {{ background: alpha({c}, 0.22); color: {c}; }}
window.home-window .device-card{scope} .indicator.active {{ background: alpha({c}, 0.18); color: {c}; }}
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_swatches_fall_back_to_the_accent() {
        let css = css(&[("amber".into(), "#ffaa00".into())], "#123456");
        assert!(css.contains("@define-color kind_light #ffaa00;"));
        assert!(css.contains("@define-color kind_camera #123456;"));
        assert!(css.contains(".device-card.kind-climate.heating.on"));
    }
}
