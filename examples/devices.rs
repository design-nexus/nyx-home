//! What `Home.Devices1` would report: counts only. Read-only.
//! `cargo run --example devices`
#[tokio::main(flavor = "current_thread")]
async fn main() {
    nyx_core::app::init(nyx_core::app::App {
        slug: "nyx-home",
        name: "Home",
    });
    let client = match nyx_home::hybrid::HybridClient::load().await {
        Ok(c) => c,
        Err(e) => return println!("not connected: {e}"),
    };
    let states = client.states().await.unwrap_or_default();
    let lists = client
        .ws_lists(&[
            "config/entity_registry/list",
            "config/device_registry/list",
            "config/area_registry/list",
        ])
        .await;
    let (room, hidden, order) = match &lists {
        Ok(l) => nyx_home::devices::rooms(&l[0], &l[1], &l[2]),
        Err(e) => {
            println!("registries: {e}");
            Default::default()
        }
    };
    let s = nyx_home::devices::snapshot(&states, &room, &hidden, &order);
    let lights = s.devices.iter().filter(|d| d.domain == "light").count();
    println!(
        "{} states; {} rooms; {} devices ({} lights, {} on); {} scenes",
        states.len(),
        s.rooms.len(),
        s.devices.len(),
        lights,
        s.devices.iter().filter(|d| d.on).count(),
        s.scenes.len()
    );
}
