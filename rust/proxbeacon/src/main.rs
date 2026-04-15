use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use axum::routing::{get, post};
use axum::Form;
use serde::Deserialize;
use slint::{Model, VecModel};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "framebuffer")]
use slint_backend_linuxfb::LinuxFbPlatformBuilder;

slint::slint! {
    export struct TrainData {
        route: string,
        destination: string,
        minutes: string,
        route_color: color,
    }

    export component App inherits Window {
        in-out property <string> station_name: "Loading...";
        in-out property <[TrainData]> trains;

        background: black;

        VerticalLayout {
            alignment: start;
            padding: 20px;
            spacing: 10px;

            Text {
                text: station_name;
                font-size: 28pt;
                font-weight: 800;
                color: white;
                horizontal-alignment: center;
            }

            Rectangle { height: 2px; background: #333333; }

            for train in trains: HorizontalLayout {
                spacing: 15px;
                height: 60px;

                Rectangle {
                    width: 50px;
                    height: 50px;
                    border-radius: 25px;
                    background: train.route_color;

                    Text {
                        text: train.route;
                        font-size: 22pt;
                        font-weight: 800;
                        color: white;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }

                Text {
                    text: train.destination;
                    font-size: 28pt;
                    font-weight: 800;
                    color: white;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }

                Text {
                    text: train.minutes;
                    font-size: 28pt;
                    font-weight: 800;
                    color: white;
                    vertical-alignment: center;
                    min-width: 80px;
                    horizontal-alignment: right;
                }
            }
        }
    }
}

#[derive(Deserialize, Clone)]
struct ScreenInfo {
    name: String,
    station_name: String,
    #[serde(default)]
    lines: Vec<String>,
    direction: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct ScreenResponse {
    station_primary_name: Option<String>,
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    trains: Vec<Train>,
}

#[derive(Deserialize)]
struct Train {
    route: String,
    primary: String,
    est_minutes: i32,
    #[serde(default)]
    #[allow(dead_code)]
    delayed: bool,
}

// TODO: Allow this at runtime
const DEFAULT_SCREEN_ID: &str = "141-1-1-C";
const API_BASE: &str = "https://helium-prod.mylirr.org";

fn route_color(route: &str) -> slint::Color {
    match route {
        "1" | "2" | "3" => slint::Color::from_rgb_u8(238, 53, 46),
        "4" | "5" | "6" | "6X" => slint::Color::from_rgb_u8(0, 147, 60),
        "7" | "7X" => slint::Color::from_rgb_u8(185, 51, 173),
        "A" | "C" | "E" => slint::Color::from_rgb_u8(0, 57, 166),
        "B" | "D" | "F" | "FX" | "M" => slint::Color::from_rgb_u8(255, 99, 25),
        "G" => slint::Color::from_rgb_u8(108, 190, 69),
        "J" | "Z" => slint::Color::from_rgb_u8(153, 102, 51),
        "L" => slint::Color::from_rgb_u8(167, 169, 172),
        "N" | "Q" | "R" | "W" => slint::Color::from_rgb_u8(252, 204, 10),
        "S" | "FS" | "GS" | "H" => slint::Color::from_rgb_u8(128, 129, 131),
        "SIR" | "SI" => slint::Color::from_rgb_u8(0, 57, 166),
        _ => slint::Color::from_rgb_u8(128, 129, 131),
    }
}

fn route_color_css(route: &str) -> &'static str {
    match route {
        "1" | "2" | "3" => "#ee352e",
        "4" | "5" | "6" | "6X" => "#00933c",
        "7" | "7X" => "#b933ad",
        "A" | "C" | "E" => "#0039a6",
        "B" | "D" | "F" | "FX" | "M" => "#ff6319",
        "G" => "#6cbe45",
        "J" | "Z" => "#996633",
        "L" => "#a7a9ac",
        "N" | "Q" | "R" | "W" => "#fccc0a",
        "S" | "FS" | "GS" | "H" => "#808183",
        "SIR" | "SI" => "#0039a6",
        _ => "#808183",
    }
}

async fn fetch_screens_list() -> Result<Vec<ScreenInfo>, reqwest::Error> {
    let url = format!("{API_BASE}/screens");
    reqwest::get(&url).await?.json().await
}

async fn fetch_screen(screen_id: &str) -> Result<ScreenResponse, reqwest::Error> {
    let url = format!("{API_BASE}/screen/{screen_id}");
    reqwest::get(&url).await?.json().await
}

// -- Web server --

struct AppState {
    screen_id: Mutex<String>,
    stations: BTreeMap<String, Vec<ScreenInfo>>,
}

type SharedState = Arc<AppState>;

const CSS: &str = r#"
body { background: #111; color: #eee; font-family: system-ui, sans-serif; margin: 0; padding: 20px; }
h1 { font-size: 1.5rem; margin-bottom: 4px; }
a { color: #00ccff; text-decoration: none; }
a:hover { text-decoration: underline; }
.current { color: #888; margin-bottom: 16px; font-size: 0.9rem; }
.search { width: 100%; max-width: 400px; padding: 10px; font-size: 1rem; background: #222; color: #eee; border: 1px solid #444; border-radius: 4px; margin-bottom: 16px; box-sizing: border-box; }
.station-list { list-style: none; padding: 0; margin: 0; }
.station-list li { padding: 10px 0; border-bottom: 1px solid #222; }
.station-list li.hidden { display: none; }
.station-name { font-size: 1.1rem; font-weight: bold; }
.lines { margin-left: 8px; }
.line-badge { display: inline-block; width: 22px; height: 22px; border-radius: 50%; text-align: center; line-height: 22px; font-size: 11px; font-weight: bold; color: white; margin-right: 3px; vertical-align: middle; }
.screen-list { list-style: none; padding: 0; margin: 0; }
.screen-list li { border-bottom: 1px solid #222; }
.screen-list a { display: block; padding: 12px 0; }
.screen-id { font-weight: bold; font-size: 1rem; }
.screen-desc { color: #888; font-size: 0.85rem; margin-top: 2px; }
.screen-dir { color: #aaa; font-size: 0.85rem; }
.back { display: inline-block; margin-bottom: 16px; }
.active { color: #00ff88; }
"#;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_line_badges(lines: &[String]) -> String {
    lines
        .iter()
        .map(|l| {
            let color = route_color_css(l);
            let text_color = if matches!(l.as_str(), "N" | "Q" | "R" | "W") {
                "black"
            } else {
                "white"
            };
            format!(
                r#"<span class="line-badge" style="background:{color};color:{text_color}">{}</span>"#,
                html_escape(l)
            )
        })
        .collect()
}

async fn web_index(State(state): State<SharedState>) -> Html<String> {
    let current = state.screen_id.lock().unwrap().clone();

    let mut stations_html = String::new();
    for (name, screens) in &state.stations {
        let mut all_lines: Vec<&str> = screens
            .iter()
            .flat_map(|s| s.lines.iter().map(|l| l.as_str()))
            .collect();
        all_lines.sort();
        all_lines.dedup();
        let badges =
            render_line_badges(&all_lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let encoded = html_escape(name);
        let url_encoded = urlencoding::encode(name);
        stations_html.push_str(&format!(
            r#"<li data-name="{lower}"><a href="/station?name={url_encoded}"><span class="station-name">{encoded}</span><span class="lines">{badges}</span></a></li>"#,
            lower = encoded.to_lowercase(),
        ));
    }

    Html(format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>proxbeacon</title><style>{CSS}</style></head>
<body>
<h1>proxbeacon</h1>
<p class="current">Current screen: <strong>{current}</strong></p>
<input type="text" class="search" id="search" placeholder="Search stations..." autofocus>
<ul class="station-list" id="stations">{stations_html}</ul>
<script>
const input = document.getElementById('search');
const items = document.querySelectorAll('#stations li');
input.addEventListener('input', () => {{
  const q = input.value.toLowerCase();
  items.forEach(li => {{
    li.classList.toggle('hidden', !li.dataset.name.includes(q));
  }});
}});
</script>
</body></html>"#
    ))
}

#[derive(Deserialize)]
struct StationQuery {
    name: String,
}

async fn web_station(
    State(state): State<SharedState>,
    Query(query): Query<StationQuery>,
) -> Html<String> {
    let current = state.screen_id.lock().unwrap().clone();
    let station_name = html_escape(&query.name);

    let screens_html = match state.stations.get(&query.name) {
        Some(screens) => {
            let mut html = String::new();
            for s in screens {
                let is_active = s.name == current;
                let active_class = if is_active { " active" } else { "" };
                let dir = s.direction.as_deref().unwrap_or("");
                let desc = s.description.as_deref().unwrap_or("");
                let badges = render_line_badges(&s.lines);
                html.push_str(&format!(
                    r#"<li><a href="/set?screen_id={id}"><span class="screen-id{active_class}">{id}</span> {badges} <span class="screen-dir">{dir}</span><div class="screen-desc">{desc}</div></a></li>"#,
                    id = html_escape(&s.name),
                    dir = html_escape(dir),
                    desc = html_escape(desc),
                ));
            }
            html
        }
        None => "<li>Station not found</li>".to_string(),
    };

    Html(format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{station_name} - proxbeacon</title><style>{CSS}</style></head>
<body>
<a href="/" class="back">&larr; Back to stations</a>
<h1>{station_name}</h1>
<p class="current">Current screen: <strong>{current}</strong></p>
<ul class="screen-list">{screens_html}</ul>
</body></html>"#
    ))
}

#[derive(Deserialize)]
struct SetScreenQuery {
    screen_id: String,
}

async fn web_set_screen(
    State(state): State<SharedState>,
    Query(query): Query<SetScreenQuery>,
) -> Redirect {
    let new_id = query.screen_id.trim().to_string();
    eprintln!("screen changed to: {new_id}");
    *state.screen_id.lock().unwrap() = new_id;
    Redirect::to("/")
}

#[derive(Deserialize)]
struct ScreenForm {
    screen_id: String,
}

async fn web_set_screen_form(
    State(state): State<SharedState>,
    Form(form): Form<ScreenForm>,
) -> Redirect {
    let new_id = form.screen_id.trim().to_string();
    eprintln!("screen changed to: {new_id}");
    *state.screen_id.lock().unwrap() = new_id;
    Redirect::to("/")
}

fn start_web_server(state: SharedState) {
    let app = axum::Router::new()
        .route("/", get(web_index))
        .route("/station", get(web_station))
        .route("/set", get(web_set_screen))
        .route("/screen", post(web_set_screen_form))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8123));
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        eprintln!("web server listening on http://{addr}");
        axum::serve(listener, app).await.unwrap();
    });
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _tokio = rt.enter();

    // Fetch screens list at startup
    eprintln!("fetching screens list...");
    let screens = rt
        .block_on(fetch_screens_list())
        .expect("failed to fetch screens list");
    let mut stations: BTreeMap<String, Vec<ScreenInfo>> = BTreeMap::new();
    for screen in screens {
        stations
            .entry(screen.station_name.clone())
            .or_default()
            .push(screen);
    }
    eprintln!("loaded {} stations", stations.len());

    let state: SharedState = Arc::new(AppState {
        screen_id: Mutex::new(DEFAULT_SCREEN_ID.to_string()),
        stations,
    });
    let screen_id = state.clone();
    start_web_server(state);

    #[cfg(feature = "framebuffer")]
    {
        let platform = LinuxFbPlatformBuilder::new()
            .with_framebuffer("/dev/fb0")
            .with_input_autodiscovery(true)
            .build()
            .unwrap();
        slint::platform::set_platform(Box::new(platform)).unwrap();
    }

    let trains_model = Rc::new(VecModel::<TrainData>::default());
    let app = App::new().unwrap();
    app.set_trains(trains_model.clone().into());

    let weak = app.as_weak();
    let model = trains_model.clone();

    slint::spawn_local(async move {
        let mut current_id = String::new();
        loop {
            let id = screen_id.screen_id.lock().unwrap().clone();

            if id != current_id {
                current_id = id.clone();
                if let Some(app) = weak.upgrade() {
                    app.set_station_name("Loading...".into());
                }
                while model.row_count() > 0 {
                    model.remove(0);
                }
            }

            match fetch_screen(&current_id).await {
                Ok(resp) => {
                    if let Some(app) = weak.upgrade() {
                        if let Some(name) = &resp.station_primary_name {
                            app.set_station_name(name.into());
                        }
                    }

                    let trains: Vec<TrainData> = resp
                        .sections
                        .iter()
                        .flat_map(|s| &s.trains)
                        .map(|t| {
                            let mins = if t.est_minutes == 0 {
                                "NOW".into()
                            } else {
                                format!("{} min", t.est_minutes).into()
                            };
                            TrainData {
                                route: t.route.clone().into(),
                                destination: t.primary.clone().into(),
                                minutes: mins,
                                route_color: route_color(&t.route),
                            }
                        })
                        .collect();

                    for (i, train) in trains.iter().enumerate() {
                        if i < model.row_count() {
                            model.set_row_data(i, train.clone());
                        } else {
                            model.push(train.clone());
                        }
                    }
                    while model.row_count() > trains.len() {
                        model.remove(model.row_count() - 1);
                    }
                }
                Err(e) => eprintln!("fetch error: {e}"),
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    })
    .unwrap();

    app.run().unwrap();
}
