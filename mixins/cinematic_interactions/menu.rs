// The jar's settings screen (CinematicSettingsScreen): "Cinematic Interactions — Settings" with a
// search box, the General, Stations, Camera and Advanced tabs, a scrolling list of options, a box
// that says what the pointed option does, and Reset all, Cancel and Done. It is opened from the
// "Cinematic Interaction Settings..." button on the Options screen and drawn in its place.

use crate::hud::{HudAtlas, HudBatch, rgba};
use crate::native_cinematic_interactions_scene as ci;
use crate::text::Text;
use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    General,
    Stations,
    Camera,
    Advanced,
}

const TABS: [(Tab, &str); 4] = [(Tab::General, "General"), (Tab::Stations, "Stations"), (Tab::Camera, "Camera"), (Tab::Advanced, "Advanced")];

/// One setting of the list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opt {
    ReachWhy,
    Hints,
    Invite,
    Orbit,
    Arms,
    Preset,
    Reduce,
    WayIn,
    Approach,
    Leaving,
    Window,
    Lean,
    Zoom,
    Bars,
    Breathing,
    Shake,
    Flashes,
    ClientOnly,
    Developer,
}

/// A line of the list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    Header(&'static str),
    Opt(Opt),
    AllOnOff,
    Stations(usize, Option<usize>),
    Note(&'static str),
    Profile(usize),
}

/// What a press does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Tab(Tab),
    Search,
    Cycle(Opt),
    Slide(Opt, f32, f32),
    Station(usize),
    AllOn,
    AllOff,
    DevReset,
    Profile,
    Reset,
    Cancel,
    Done,
}

struct Menu {
    open: bool,
    tab: Tab,
    scroll: f32,
    search: String,
    typing: bool,
    edit: ci::Config,
    drag: Option<(Opt, f32, f32)>,
}

static MENU: Mutex<Menu> = Mutex::new(Menu {
    open: false,
    tab: Tab::General,
    scroll: 0.0,
    search: String::new(),
    typing: false,
    edit: ci::DEFAULT,
    drag: None,
});

fn menu() -> std::sync::MutexGuard<'static, Menu> {
    MENU.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn is_open() -> bool {
    menu().open
}

/// The button on the Options screen: the screen opens on its first tab with the saved settings.
pub fn open(dir: Option<std::path::PathBuf>) {
    if !ci::loaded() {
        ci::load(dir);
    }
    let edit = ci::config();
    let mut m = menu();
    m.open = true;
    m.tab = Tab::General;
    m.scroll = 0.0;
    m.search.clear();
    m.typing = false;
    m.drag = None;
    m.edit = edit;
}

pub fn close() {
    let mut m = menu();
    m.open = false;
    m.drag = None;
    m.typing = false;
}

fn label(o: Opt) -> &'static str {
    match o {
        Opt::ReachWhy => "When a station can't be reached",
        Opt::Hints => "Control hints",
        Opt::Invite => "Campfire invitation",
        Opt::Orbit => "Orbit view key",
        Opt::Arms => "Arms at work",
        Opt::Preset => "Preset",
        Opt::Reduce => "Reduce motion",
        Opt::WayIn => "Way in",
        Opt::Approach => "Approach speed",
        Opt::Leaving => "Leaving speed",
        Opt::Window => "Quick return window",
        Opt::Lean => "Camera lean",
        Opt::Zoom => "Zoom",
        Opt::Bars => "Letterbox bars",
        Opt::Breathing => "Breathing camera",
        Opt::Shake => "Shake at big moments",
        Opt::Flashes => "Screen flashes",
        Opt::ClientOnly => "Client-only mode",
        Opt::Developer => "Developer mode",
    }
}

fn describe(o: Opt) -> &'static str {
    match o {
        Opt::ReachWhy => "A station out of reach says why (too far, too low, too high), or nothing happens.",
        Opt::Hints => "The line of controls at the bottom of the screen while you work at a station.",
        Opt::Invite => "The \"Sit by the fire\" plate under the crosshair when you look at a campfire.",
        Opt::Orbit => "Hold the orbit key to look around the station, or press it once to turn the orbit on and off.",
        Opt::Arms => "How far the arms reach out while they work: stretched to the spot, or relaxed nearer the body.",
        Opt::Preset => "A set of camera speeds: Balanced, Cinematic (slower), Snappy (faster) or your own.",
        Opt::Reduce => "Fewer camera moves: no black bars, no lean and a quicker glide.",
        Opt::WayIn => "Coming back to a station you just left: a quick return, or always the full glide.",
        Opt::Approach => "How fast the camera glides to the station.",
        Opt::Leaving => "How fast the camera glides back to you.",
        Opt::Window => "How long after leaving a station coming back to it is quick.",
        Opt::Lean => "How much the camera leans over the station on the way in.",
        Opt::Zoom => "How far the view zooms in on the station.",
        Opt::Bars => "The black bars at the top and bottom while the camera moves.",
        Opt::Breathing => "The camera moves a little, as you breathe, while you work.",
        Opt::Shake => "The camera shakes when a hammer strikes or a beacon lights.",
        Opt::Flashes => "The bright light of a ritual: an enchanting, a beacon lighting, a forge.",
        Opt::ClientOnly => "Only for servers without the mod. Not needed in Horizon.",
        Opt::Developer => "Shows the layout profiles of the stations.",
    }
}

const NONE_TEXT: &str = "Point at an option to see what it does.";

/// The value on an option's button, and whether it is a slider (with its 0..1 place).
fn value(c: &ci::Config, o: Opt) -> (String, Option<f32>) {
    let on = |b: bool, yes: &str, no: &str| if b { format!("\u{a7}a{yes}") } else { no.to_owned() };
    let pct = |v: u16, lo: f32, hi: f32| (format!("{v}%"), Some(((f32::from(v) - lo) / (hi - lo)).clamp(0.0, 1.0)));
    match o {
        Opt::ReachWhy => ((if c.reach_why { "Show why" } else { "Silent" }).to_owned(), None),
        Opt::Hints => (on(c.hints, "Shown", "Hidden"), None),
        Opt::Invite => (on(c.invite, "Shown", "Hidden"), None),
        Opt::Orbit => ((if c.orbit_hold { "Hold" } else { "Toggle" }).to_owned(), None),
        Opt::Arms => ((if c.arms_stretched { "Stretched" } else { "Relaxed" }).to_owned(), None),
        Opt::Preset => ((["Balanced", "Cinematic", "Snappy", "Custom"][usize::from(c.preset.min(3))]).to_owned(), None),
        Opt::Reduce => (on(c.reduce_motion, "On", "Off"), None),
        Opt::WayIn => ((if c.quick_return { "Quick return" } else { "Full glide" }).to_owned(), None),
        Opt::Approach => pct(c.approach, 25.0, 200.0),
        Opt::Leaving => pct(c.leaving, 25.0, 200.0),
        Opt::Window => (format!("{} s", c.window), Some((f32::from(c.window) / 30.0).clamp(0.0, 1.0))),
        Opt::Lean => pct(c.lean, 0.0, 100.0),
        Opt::Zoom => pct(c.zoom, 0.0, 100.0),
        Opt::Bars => (on(c.bars, "On", "Off"), None),
        Opt::Breathing => (on(c.breathing, "On", "Off"), None),
        Opt::Shake => (on(c.shake, "On", "Off"), None),
        Opt::Flashes => (on(c.flashes, "On", "Off"), None),
        Opt::ClientOnly => ("Off".to_owned(), None),
        Opt::Developer => (on(c.developer, "On", "Off"), None),
    }
}

/// A press on an option's button: the next value.
fn cycle(c: &mut ci::Config, o: Opt) {
    match o {
        Opt::ReachWhy => c.reach_why = !c.reach_why,
        Opt::Hints => c.hints = !c.hints,
        Opt::Invite => c.invite = !c.invite,
        Opt::Orbit => c.orbit_hold = !c.orbit_hold,
        Opt::Arms => c.arms_stretched = !c.arms_stretched,
        Opt::Preset => {
            c.preset = (c.preset + 1) % 3;
            let (a, l) = [(100, 100), (70, 80), (150, 160)][usize::from(c.preset)];
            c.approach = a;
            c.leaving = l;
        }
        Opt::Reduce => c.reduce_motion = !c.reduce_motion,
        Opt::WayIn => c.quick_return = !c.quick_return,
        Opt::Bars => c.bars = !c.bars,
        Opt::Breathing => c.breathing = !c.breathing,
        Opt::Shake => c.shake = !c.shake,
        Opt::Flashes => c.flashes = !c.flashes,
        Opt::Developer => c.developer = !c.developer,
        _ => {}
    }
}

/// A slider set at `k` (0..1).
fn set_slider(c: &mut ci::Config, o: Opt, k: f32) {
    let k = k.clamp(0.0, 1.0);
    let pct = |lo: f32, hi: f32| (((lo + (hi - lo) * k) / 5.0).round() * 5.0) as u16;
    match o {
        Opt::Approach => c.approach = pct(25.0, 200.0),
        Opt::Leaving => c.leaving = pct(25.0, 200.0),
        Opt::Window => c.window = (30.0 * k).round() as u16,
        Opt::Lean => c.lean = pct(0.0, 100.0),
        Opt::Zoom => c.zoom = pct(0.0, 100.0).max(1),
        _ => return,
    }
    if matches!(o, Opt::Approach | Opt::Leaving) {
        c.preset = 3;
    }
}

const PROFILE_ORDER: [usize; 18] = [0, 5, 6, 7, 12, 9, 10, 11, 2, 3, 4, 13, 1, 8, 15, 14, 16, 17];

fn rows(tab: Tab, c: &ci::Config) -> Vec<Row> {
    let mut out = Vec::new();
    match tab {
        Tab::General => {
            for o in [Opt::ReachWhy, Opt::Hints, Opt::Invite, Opt::Orbit, Opt::Arms] {
                out.push(Row::Opt(o));
            }
        }
        Tab::Stations => {
            out.push(Row::AllOnOff);
            for (g, name) in ci::GROUPS.iter().enumerate() {
                out.push(Row::Header(name));
                let list: Vec<usize> = (0..ci::STATIONS.len()).filter(|i| usize::from(ci::STATIONS[*i].3) == g).collect();
                for pair in list.chunks(2) {
                    out.push(Row::Stations(pair[0], pair.get(1).copied()));
                }
            }
        }
        Tab::Camera => {
            out.push(Row::Opt(Opt::Preset));
            out.push(Row::Opt(Opt::Reduce));
            out.push(Row::Header("Pace"));
            for o in [Opt::WayIn, Opt::Approach, Opt::Leaving, Opt::Window] {
                out.push(Row::Opt(o));
            }
            out.push(Row::Header("View"));
            out.push(Row::Opt(Opt::Lean));
            out.push(Row::Opt(Opt::Zoom));
            out.push(Row::Header("Effects"));
            for o in [Opt::Bars, Opt::Breathing, Opt::Shake, Opt::Flashes] {
                out.push(Row::Opt(o));
            }
        }
        Tab::Advanced => {
            out.push(Row::Opt(Opt::ClientOnly));
            out.push(Row::Opt(Opt::Developer));
            if c.developer {
                out.push(Row::Header("Layout profiles"));
                for i in PROFILE_ORDER {
                    out.push(Row::Profile(i));
                }
            } else {
                out.push(Row::Note("Turn on developer mode to choose layout profiles."));
            }
        }
    }
    out
}

/// The rows that match the search (a header stays while a row under it matches).
fn filtered(rows: Vec<Row>, query: &str) -> Vec<Row> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return rows;
    }
    let hit = |r: &Row| match r {
        Row::Opt(o) => label(*o).to_lowercase().contains(&q),
        Row::Stations(a, b) => ci::STATIONS[*a].1.to_lowercase().contains(&q) || b.is_some_and(|b| ci::STATIONS[b].1.to_lowercase().contains(&q)),
        Row::Profile(i) => ci::STATIONS[*i].1.to_lowercase().contains(&q),
        _ => false,
    };
    let mut out: Vec<Row> = Vec::new();
    let mut header = None;
    for r in rows {
        if let Row::Header(_) = r {
            header = Some(r);
        } else if hit(&r) {
            if let Some(h) = header.take() {
                out.push(h);
            }
            out.push(r);
        }
    }
    out
}

fn row_height(r: &Row, first: bool) -> f32 {
    match r {
        Row::Header(_) if !first => 30.0,
        Row::Header(_) => 22.0,
        Row::Note(_) => 22.0,
        _ => 24.0,
    }
}

/// Where everything is (GUI pixels): the list's top and bottom.
fn list_area(h: f32) -> (f32, f32) {
    (74.0, h - 92.0)
}

/// A drawn piece of the screen.
enum Piece {
    Button { x: f32, y: f32, w: f32, text: String, active: bool, act: Option<Act>, icon: Option<&'static str> },
    Slider { x: f32, y: f32, w: f32, text: String, k: f32, opt: Opt },
    Label { x: f32, y: f32, text: String, color: u32 },
}

/// The list's pieces at the scroll, and its content height.
fn list(m: &Menu, w: f32, h: f32) -> (Vec<(Piece, Option<Opt>)>, f32) {
    let cx = (w / 2.0).floor();
    let (top, _) = list_area(h);
    let c = &m.edit;
    let rows = filtered(rows(m.tab, c), &m.search);
    let mut out = Vec::new();
    let mut y = top + 4.0 - m.scroll;
    let mut content = 8.0;
    for (i, r) in rows.iter().enumerate() {
        let rh = row_height(r, i == 0);
        let ry = y + rh - 24.0;
        match *r {
            Row::Header(t) => out.push((Piece::Label { x: cx - 150.0, y: ry + 10.0, text: t.to_owned(), color: rgba(224, 196, 120, 255) }, None)),
            Row::Note(t) => out.push((Piece::Label { x: cx - 150.0, y: y + 8.0, text: t.to_owned(), color: rgba(160, 160, 160, 255) }, None)),
            Row::Opt(o) => {
                let disabled = o == Opt::ClientOnly;
                let color = if disabled { rgba(160, 160, 160, 255) } else { u32::MAX };
                out.push((Piece::Label { x: cx - 150.0, y: ry + 6.0, text: label(o).to_owned(), color }, Some(o)));
                let (text, k) = value(c, o);
                if let Some(k) = k {
                    out.push((Piece::Slider { x: cx + 9.0, y: ry, w: 120.0, text, k, opt: o }, Some(o)));
                } else {
                    out.push((Piece::Button { x: cx + 9.0, y: ry, w: 120.0, text, active: !disabled, act: (!disabled).then_some(Act::Cycle(o)), icon: None }, Some(o)));
                }
                if o == Opt::Developer && c.developer {
                    out.push((Piece::Button { x: cx + 134.0, y: ry, w: 20.0, text: "\u{21ba}".to_owned(), active: true, act: Some(Act::DevReset), icon: None }, Some(o)));
                }
            }
            Row::AllOnOff => {
                out.push((Piece::Label { x: cx - 150.0, y: ry + 6.0, text: "A station turned off opens".to_owned(), color: rgba(160, 160, 160, 255) }, None));
                out.push((Piece::Button { x: cx - 7.0, y: ry, w: 77.0, text: "All on".to_owned(), active: true, act: Some(Act::AllOn), icon: None }, None));
                out.push((Piece::Button { x: cx + 75.0, y: ry, w: 77.0, text: "All off".to_owned(), active: true, act: Some(Act::AllOff), icon: None }, None));
            }
            Row::Stations(a, b) => {
                for (k, i) in [Some(a), b].into_iter().enumerate() {
                    let Some(i) = i else { continue };
                    let st = ci::STATIONS[i];
                    let on = c.off & (1 << i) == 0;
                    let text = format!("{} \u{b7} {}", st.1, if on { "\u{a7}aOn" } else { "\u{a7}cOff" });
                    let x = if k == 0 { cx - 152.0 } else { cx + 3.0 };
                    out.push((Piece::Button { x, y: ry, w: 149.0, text, active: true, act: Some(Act::Station(i)), icon: Some(st.2) }, None));
                }
            }
            Row::Profile(i) => {
                out.push((Piece::Label { x: cx - 150.0, y: ry + 6.0, text: ci::STATIONS[i].1.to_owned(), color: u32::MAX }, None));
                out.push((Piece::Button { x: cx + 9.0, y: ry, w: 120.0, text: "Profile 1".to_owned(), active: true, act: Some(Act::Profile), icon: None }, None));
            }
        }
        y += rh;
        content += rh;
    }
    (out, content)
}

/// The fixed pieces: the search box's place, the tabs and the three buttons at the bottom.
fn frame(m: &Menu, w: f32, h: f32) -> Vec<Piece> {
    let cx = (w / 2.0).floor();
    let mut out = Vec::new();
    for (i, (tab, name)) in TABS.iter().enumerate() {
        let x = cx - 178.0 + i as f32 * 90.0;
        let here = *tab == m.tab;
        out.push(Piece::Button { x, y: 50.0, w: 87.0, text: (*name).to_owned(), active: !here, act: Some(Act::Tab(*tab)), icon: None });
    }
    for (i, (name, act)) in [("Reset all", Act::Reset), ("Cancel", Act::Cancel), ("Done", Act::Done)].iter().enumerate() {
        out.push(Piece::Button { x: cx - 154.0 + i as f32 * 106.0, y: h - 25.0, w: 98.0, text: (*name).to_owned(), active: true, act: Some(*act), icon: None });
    }
    out
}

fn inside(x: f32, y: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

/// The longest start of `text` that fits in `w`.
fn clip(atlas: &HudAtlas, text: &str, w: f32) -> String {
    if HudBatch::text_width(atlas, text) <= w {
        return text.to_owned();
    }
    let mut s: String = text.to_owned();
    while !s.is_empty() && HudBatch::text_width(atlas, &s) > w {
        s.pop();
    }
    s
}

/// Words wrapped to `w`.
fn wrap(atlas: &HudAtlas, text: &str, w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let next = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
        if HudBatch::text_width(atlas, &next) > w && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_owned();
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The screen, over the blurred world: `icon` draws a station's item at (x, y).
pub fn draw(atlas: &HudAtlas, batch: &mut HudBatch, cursor: (f32, f32), icon: &mut dyn FnMut(&mut HudBatch, &str, f32, f32)) {
    let m = menu();
    let (w, h) = (batch.width.ceil(), batch.height.ceil());
    let cx = (w / 2.0).floor();
    let (mx, my) = cursor;
    let (top, bottom) = list_area(h);
    // Title and search box.
    let title = "Cinematic Interactions \u{2014} Settings";
    batch.text(atlas, title, (cx - HudBatch::text_width(atlas, title) / 2.0).round(), 14.0, u32::MAX, true);
    let border = if m.typing { u32::MAX } else { rgba(160, 160, 160, 255) };
    batch.fill(atlas, cx - 100.0, 28.0, 200.0, 17.0, border);
    batch.fill(atlas, cx - 99.0, 29.0, 198.0, 15.0, rgba(0, 0, 0, 255));
    if m.search.is_empty() && !m.typing {
        batch.text(atlas, "Search settings", cx - 96.0, 33.0, rgba(112, 112, 112, 255), true);
    } else {
        let caret = if m.typing && (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis()) / 300) % 2 == 0 { "_" } else { "" };
        batch.text(atlas, &format!("{}{caret}", clip(atlas, &m.search, 188.0)), cx - 96.0, 33.0, rgba(224, 224, 224, 255), true);
    }
    // The list band and its separators.
    batch.fill(atlas, 0.0, top, w, bottom - top, rgba(0, 0, 0, 90));
    crate::ui::separators(batch, atlas, top, bottom, true);
    let mut hovered: Option<Opt> = None;
    let (pieces, content) = list(&m, w, h);
    let start = batch.vertices.len();
    let in_list = my >= top && my < bottom;
    for (p, opt) in &pieces {
        draw_piece(atlas, batch, p, cursor, in_list, m.drag.map(|d| d.0), icon);
        if in_list && let Some(o) = opt {
            let (x0, y0, ww) = match p {
                Piece::Label { x, y, .. } => (*x, *y - 6.0, 150.0),
                Piece::Button { x, y, w, .. } | Piece::Slider { x, y, w, .. } => (*x, *y, *w),
            };
            if inside(mx, my, x0, y0, ww, 20.0) {
                hovered = Some(*o);
            }
        }
    }
    batch.clip_since(start, 0.0, top, w, bottom);
    if content > bottom - top {
        crate::ui::scrollbar(batch, atlas, cx + 161.0, top, bottom, content, m.scroll);
    }
    for p in frame(&m, w, h) {
        draw_piece(atlas, batch, &p, cursor, true, None, icon);
    }
    // What the pointed option does.
    let (bx, by, bw, bh) = (cx - 154.0, h - 87.0, 308.0, 55.0);
    batch.fill(atlas, bx, by, bw, bh, rgba(160, 160, 160, 255));
    batch.fill(atlas, bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0, rgba(16, 16, 16, 200));
    let text = hovered.map_or(NONE_TEXT, describe);
    for (i, line) in wrap(atlas, text, bw - 10.0).iter().take(5).enumerate() {
        batch.text(atlas, line, bx + 5.0, by + 5.0 + i as f32 * 10.0, rgba(160, 160, 160, 255), true);
    }
}

fn draw_piece(atlas: &HudAtlas, batch: &mut HudBatch, p: &Piece, (mx, my): (f32, f32), live: bool, dragging: Option<Opt>, icon: &mut dyn FnMut(&mut HudBatch, &str, f32, f32)) {
    match p {
        Piece::Label { x, y, text, color } => {
            let t = clip(atlas, text, 152.0);
            batch.text(atlas, &t, *x, *y, *color, true);
        }
        Piece::Button { x, y, w, text, active, icon: item, .. } => {
            let hover = live && *active && inside(mx, my, *x, *y, *w, 20.0);
            let label = Text::legacy(text);
            if let Some(item) = item {
                // The station's item at the left, the label in the room right of it.
                crate::ui::button(batch, atlas, *x, *y, *w, 20.0, &Text::literal(String::new()), *active, hover);
                icon(batch, item, *x + 2.0, *y + 2.0);
                crate::ui::centered_text(batch, atlas, &label, *x + 20.0, *y + 6.0, *w - 22.0, u32::MAX);
            } else {
                crate::ui::button(batch, atlas, *x, *y, *w, 20.0, &label, *active, hover);
            }
        }
        Piece::Slider { x, y, w, text, k, opt } => {
            let hover = live && inside(mx, my, *x, *y, *w, 20.0);
            crate::ui::slider(batch, atlas, *x, *y, *w, &Text::literal(text.clone()), *k, hover, true, dragging == Some(*opt));
        }
    }
}

/// What a press at (x, y) did.
pub enum Out {
    None,
    Click,
    Close,
}

/// A press (or a release) on the screen.
pub fn mouse(atlas: &HudAtlas, w: f32, h: f32, x: f32, y: f32, pressed: bool) -> Out {
    let mut m = menu();
    if !pressed {
        m.drag = None;
        return Out::None;
    }
    let cx = (w / 2.0).floor();
    let (top, bottom) = list_area(h);
    m.typing = inside(x, y, cx - 100.0, 28.0, 200.0, 17.0);
    if m.typing {
        return Out::Click;
    }
    let mut act = None;
    for p in frame(&m, w, h) {
        if let Piece::Button { x: bx, y: by, w: bw, act: Some(a), .. } = p
            && inside(x, y, bx, by, bw, 20.0)
        {
            act = Some(a);
        }
    }
    if act.is_none() && y >= top && y < bottom {
        let (pieces, _) = list(&m, w, h);
        for (p, _) in pieces {
            match p {
                Piece::Button { x: bx, y: by, w: bw, act: Some(a), .. } if inside(x, y, bx, by, bw, 20.0) => act = Some(a),
                Piece::Slider { x: bx, y: by, w: bw, opt, .. } if inside(x, y, bx, by, bw, 20.0) => act = Some(Act::Slide(opt, bx, bw)),
                _ => {}
            }
        }
    }
    let _ = atlas;
    let Some(act) = act else {
        return Out::None;
    };
    match act {
        Act::Tab(t) => {
            m.tab = t;
            m.scroll = 0.0;
        }
        Act::Search => m.typing = true,
        Act::Cycle(o) => cycle(&mut m.edit, o),
        Act::Slide(o, sx, sw) => {
            m.drag = Some((o, sx, sw));
            set_slider(&mut m.edit, o, crate::ui::slider_value(sx, sw, x));
            return Out::None;
        }
        Act::Station(i) => m.edit.off ^= 1 << i,
        Act::AllOn => m.edit.off = 0,
        Act::AllOff => m.edit.off = (1 << ci::STATIONS.len()) - 1,
        Act::DevReset => m.edit.developer = false,
        Act::Profile => {}
        Act::Reset => {
            m.edit = ci::DEFAULT;
            m.scroll = 0.0;
        }
        Act::Cancel => {
            m.open = false;
            return Out::Close;
        }
        Act::Done => {
            let c = m.edit;
            m.open = false;
            drop(m);
            ci::apply_config(c);
            return Out::Close;
        }
    }
    Out::Click
}

/// The pointer moving with the button down: a slider follows it.
pub fn drag(x: f32) {
    let mut m = menu();
    if let Some((o, sx, sw)) = m.drag {
        set_slider(&mut m.edit, o, crate::ui::slider_value(sx, sw, x));
    }
}

/// The wheel: the list scrolls.
pub fn scroll(w: f32, h: f32, amount: f32) {
    let mut m = menu();
    let (top, bottom) = list_area(h);
    let (_, content) = list(&m, w, h);
    let max = (content - (bottom - top)).max(0.0);
    m.scroll = (m.scroll - amount * 12.0).clamp(0.0, max);
}

/// A key: typed text into the search box, Backspace, Escape closes (Cancel). True when the
/// screen closed.
pub fn key(escape: bool, backspace: bool, text: Option<&str>) -> bool {
    let mut m = menu();
    if escape {
        if m.typing {
            m.typing = false;
            return false;
        }
        m.open = false;
        return true;
    }
    if !m.typing {
        return false;
    }
    if backspace {
        m.search.pop();
    } else if let Some(t) = text {
        m.search.extend(t.chars().filter(|c| !c.is_control()));
    }
    m.scroll = 0.0;
    false
}
