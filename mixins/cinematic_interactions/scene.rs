// Cinematic Interactions for Horizon: right-click a crafting table and the camera glides over it,
// your own arms rest on the table and reach for the cell under the cursor, the items lie on the
// 3x3 grid as small blocks, the inventory sits in a panel at the top of the screen and the
// controls are listed at the bottom. Black bars slide in and out while the camera moves.
//
// Written from scratch for Horizon, after what Cinematic Interactions by Southware (MIT) does for
// Java. The crafting menu itself is Horizon's: the hooks only say which slot is under the cursor
// (a cell on the table, a slot of the panel) and draw the scene, so every click, drag, shift-click
// and the server's answers work as they do in the normal screen.
//
// Everything here is plain logic and drawing; the hooks in the game bin hand it the game's state.

use crate::hud::{HudAtlas, HudBatch, rgba};
use glam::{DVec3, Mat4, Vec3};
use std::sync::Mutex;
use std::time::Instant;

pub const ID: &str = "cinematic_interactions";
pub const NAME: &str = "Cinematic Interactions";
pub const VERSION: &str = "0.8.0";
pub const DESCRIPTION: &str = "By tariksabanuz. Right-click a crafting table: you walk up, the camera leans over it, your arms reach \
for the grid and the items lie on the table, and stay there when you leave. Furnaces, smithing and cartography tables, \
grindstones, brewing stands, looms, enchanting tables, beacons and chiseled bookshelves are worked in the world too; \
press H at a campfire to sit by the fire. Sneak + right-click opens the normal screen.";

/// A camera pose: where the eye is, its yaw and its pitch (degrees, Minecraft's convention).
pub type Pose = (DVec3, f32, f32);

// ------------------------------------------------------------------ settings

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// The black bars while the camera moves.
    pub bars: bool,
    /// The control hints at the bottom of the screen.
    pub hints: bool,
    /// The camera's speed: 0 slow, 1 normal, 2 fast.
    pub speed: u8,
}

pub const DEFAULT: Config = Config { bars: true, hints: true, speed: 1 };
const SPEEDS: [&str; 3] = ["Slow", "Normal", "Fast"];

pub fn parse(text: &str) -> Config {
    let mut c = DEFAULT;
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        let on = v == "true";
        match k {
            "bars" => c.bars = on,
            "hints" => c.hints = on,
            "speed" => c.speed = v.parse::<u8>().unwrap_or(1).min(2),
            _ => {}
        }
    }
    c
}

pub fn to_text(c: &Config) -> String {
    format!("bars={}\nhints={}\nspeed={}\n", c.bars, c.hints, c.speed)
}

/// Seconds the camera takes to lean over the table (after the walk), and to come back.
pub fn durations(c: &Config) -> (f32, f32) {
    let speed = match c.speed {
        0 => 0.6,
        2 => 1.6,
        _ => 1.0,
    };
    (LEAN / speed, OUTRO / speed)
}

/// The lean over the table takes 1.2 seconds, the way back 1.05.
pub const LEAN: f32 = 1.2;
pub const OUTRO: f32 = 1.05;
/// The walk covers this share of the intro's progress; the lean the rest.
pub const WALK_SHARE: f32 = 0.3;

fn window(x: f32, a: f32, b: f32) -> f32 {
    ((x - a) / (b - a)).clamp(0.0, 1.0)
}

fn sine(x: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * x.clamp(0.0, 1.0)).cos()
}

fn hump(x: f32) -> f32 {
    (std::f32::consts::PI * x.clamp(0.0, 1.0)).sin()
}

// ------------------------------------------------------------------ state

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// No scene: the game's own camera and hands.
    Idle,
    /// The camera glides to the table.
    Enter,
    /// At the table.
    Inside,
    /// The camera glides back to the player.
    Exit,
}

/// What is under the cursor in the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hover {
    /// A cell of the 3x3 grid (0 is the far left one, as the menu counts).
    Cell(usize),
    /// The crafted item floating over the table.
    Result,
    /// Somewhere else on the table top.
    Table(DVec3),
}

struct State {
    config: Config,
    loaded: bool,
    active: bool,
    dir: Option<std::path::PathBuf>,
    phase: Phase,
    since: Option<Instant>,
    table: [i32; 3],
    yaw: f32,
    /// Where the glide starts.
    from: Option<Pose>,
    /// The pose shown last frame.
    last: Option<Pose>,
    /// The world field of view and aspect of last frame.
    lens: (f32, f32),
    /// The table clicked last, when, and whether it was a sneak click (the normal screen).
    clicked: Option<([i32; 3], Instant, bool)>,
    /// This screen shows as the normal one (sneak click, the recipe book button).
    plain: bool,
    /// The hand tips, left and right, and when they were stepped.
    hands: [Option<DVec3>; 2],
    /// The hands' velocities (they follow their targets on springs) and whether a stack is held.
    speed: [DVec3; 2],
    carrying: bool,
    stepped: Option<Instant>,
    /// The hand that reaches (0 left, 1 right).
    reaching: Option<usize>,
    expanded: bool,
    result_shown: bool,
    toast: Option<(&'static str, Instant)>,
    /// The walk to the table before the camera leans over it: from, to, seconds.
    walk: Option<(Pose, Pose, f32)>,
    /// Where the player stands at the table (feet), once the walk is done.
    arrived: Option<(DVec3, f32)>,
    /// The recipe chosen in the panel (the book's entry index).
    selected: Option<usize>,
    book_open: bool,
    gui: (f32, f32),
    kind: Kind,
    clicked_kind: Kind,
    /// A furnace is worked from its front: the yaw that looks at it, given with the click.
    clicked_yaw: Option<f32>,
    /// Sitting by a campfire (the H key): its food spots (CampfireStation.vanillaSpot, as spots of
    /// the scene) and a spot the carried food is to be put on.
    sitting: bool,
    campfire_spots: &'static [Spot],
    #[allow(clippy::type_complexity)]
    place: Option<([i32; 3], Option<([f64; 3], [i32; 3])>, bool)>,
    /// The recipe panel's own filter ("Have materials"), and its list: a key of what it was built
    /// from, and (book entry, craftable) in the panel's order.
    book_filter: bool,
    book_page: usize,
    book_list: (u64, Vec<(usize, bool)>),
    /// The steps the hands take laying a recipe out.
    lay_plan: Vec<LayStep>,
    lay_done: usize,
    /// "Laying out": when it began, the cells the hands visit, the recipe and whether it is a Shift.
    lay: Option<(Instant, Vec<usize>, usize, bool)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    config: DEFAULT,
    loaded: false,
    active: true,
    dir: None,
    phase: Phase::Idle,
    since: None,
    table: [0; 3],
    yaw: 0.0,
    from: None,
    last: None,
    lens: (70.0, 16.0 / 9.0),
    clicked: None,
    plain: false,
    hands: [None, None],
    speed: [DVec3::ZERO; 2],
    carrying: false,
    stepped: None,
    reaching: None,
    expanded: false,
    result_shown: false,
    toast: None,
    walk: None,
    arrived: None,
    selected: None,
    book_open: false,
    gui: (480.0, 270.0),
    kind: Kind::Table,
    clicked_kind: Kind::Table,
    clicked_yaw: None,
    book_filter: false,
    book_page: 0,
    book_list: (0, Vec::new()),
    lay_plan: Vec::new(),
    lay_done: 0,
    lay: None,
    sitting: false,
    campfire_spots: &[],
    place: None,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn loaded() -> bool {
    state().loaded
}

pub fn config() -> Config {
    state().config
}

pub fn set_config(c: Config) {
    state().config = c;
}

/// The mod's switch in Options > Mods.
pub fn set_active(on: bool) {
    let mut s = state();
    s.active = on;
    if !on {
        s.phase = Phase::Idle;
        s.plain = false;
    }
}

pub fn is_active() -> bool {
    state().active
}

pub fn load(dir: Option<std::path::PathBuf>) {
    let mut s = state();
    s.loaded = true;
    s.config = dir
        .as_ref()
        .and_then(|d| std::fs::read_to_string(d.join("cinematic_interactions.txt")).ok())
        .map_or(DEFAULT, |t| parse(&t));
    s.dir = dir;
}

fn save(s: &State) {
    if let Some(d) = &s.dir {
        let _ = std::fs::create_dir_all(d);
        let _ = std::fs::write(d.join("cinematic_interactions.txt"), to_text(&s.config));
    }
}

pub fn setting_labels() -> Vec<String> {
    let c = config();
    let onoff = |b: bool| if b { "ON" } else { "OFF" };
    vec![
        format!("Cinematic bars: {}", onoff(c.bars)),
        format!("Control hints: {}", onoff(c.hints)),
        format!("Camera speed: {}", SPEEDS[c.speed.min(2) as usize]),
    ]
}

pub fn setting_click(button: usize) {
    let mut s = state();
    match button {
        0 => s.config.bars = !s.config.bars,
        1 => s.config.hints = !s.config.hints,
        2 => s.config.speed = (s.config.speed + 1) % 3,
        _ => return,
    }
    save(&s);
}

// Re-entry guards for the hooks that call the function they wrap (1 camera, 2 hands, 4 HUD).
thread_local! {
    static GUARD: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// True when already inside `bit`'s hook; otherwise marks it.
pub fn enter(bit: u8) -> bool {
    GUARD.with(|g| {
        let v = g.get();
        if v & bit != 0 {
            true
        } else {
            g.set(v | bit);
            false
        }
    })
}

pub fn leave(bit: u8) {
    GUARD.with(|g| g.set(g.get() & !bit));
}

// ------------------------------------------------------------------ the table and the camera

/// The workstations the scene knows, each with the camera the jar gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Table,
    Furnace,
    Smoker,
    BlastFurnace,
    Anvil,
    Grindstone,
    Smithing,
    Stonecutter,
    Loom,
    Cartography,
    Enchanting,
    Brewing,
    Lectern,
    Jukebox,
    Crafter,
    Beacon,
    Campfire,
    Shelf,
}

/// The eye and the look point of a station, relative to the block: how far in front of its middle
/// the eye is (`back`), how high above the reference height `top` (`up`), the same for the look
/// point, and how far the player stands from the block's middle.
#[derive(Clone, Copy, Debug)]
pub struct Rig {
    pub top: f64,
    pub back: f64,
    pub up: f64,
    pub lat: f64,
    pub look_back: f64,
    pub look_up: f64,
    pub look_lat: f64,
    pub stand: f64,
}

const fn rig(top: f64, back: f64, up: f64, lat: f64, look_back: f64, look_up: f64, look_lat: f64, stand: f64) -> Rig {
    Rig { top, back, up, lat, look_back, look_up, look_lat, stand }
}

impl Kind {
    pub fn rig(self) -> Rig {
        match self {
            Kind::Table => rig(1.0, 0.86, 0.72, 0.0, 0.86 - 0.9, 0.0, 0.0, 0.86),
            // The furnace's staging, from the middle of its front face: the furnace and the smoker
            // are worked crouching (LOW: out 0.9, up 0.5, side 0.1, look at 0.44), the blast
            // furnace standing (TALL: 0.7, 1.15, 0.08, 0.97).
            Kind::Furnace | Kind::Smoker => rig(0.5, 0.5 + 0.9, 0.5, 0.1, 0.5, 0.44 - 0.5, 0.0, 1.15),
            Kind::BlastFurnace => rig(0.5, 0.5 + 0.7, 1.15, 0.08, 0.5, 0.97 - 0.5, 0.0, 1.15),
            Kind::Anvil => rig(1.0, 0.66, 0.7, 0.0, -0.06, 0.12, 0.0, 0.86),
            Kind::Grindstone => rig(0.5, 0.95, 1.0, 0.0, 0.0, 0.32, 0.0, 0.9),
            Kind::Smithing => rig(1.0, 0.7, 0.62, 0.0, 0.7 - 0.74, 0.0, 0.0, 0.86),
            Kind::Stonecutter => rig(0.5625, 1.0, 0.95, 0.0, -0.05, 0.25, 0.0, 0.95),
            Kind::Loom => rig(1.0, 0.8, 0.72, 0.08, -0.02, 0.05, 0.12, 0.86),
            Kind::Cartography => rig(1.0, 0.7, 0.64, 0.0, 0.7 - 0.72, 0.0, 0.0, 0.86),
            Kind::Enchanting => rig(0.75, 1.15, 1.0, 0.0, 0.0, 0.5, 0.0, 0.86),
            Kind::Brewing => rig(0.0, 1.2, 0.88, 0.0, 0.0, 0.46, 0.0, 0.95),
            Kind::Lectern => rig(1.0, 0.66, 0.0, 0.0, -0.012, -0.0, 0.0, 0.98),
            Kind::Jukebox => rig(1.0, 0.8, 0.58, 0.0, 0.04, 0.2, 0.0, 0.88),
            Kind::Crafter => rig(1.0, 0.66, 0.62, 0.0, -0.04, 0.04, 0.0, 0.9),
            // BeaconStation.lean, pulled back a little (0.3) so the powers fit over it.
            Kind::Beacon => rig(1.0, 1.5 + 1.7 * 0.3, 0.62 + 1.5 * 0.3, 0.0, 0.0, 0.42 - 0.85 * 0.3, 0.0, 1.05),
            // CampfireStation.lean: sitting 1.6 out from the fire, the eye 0.8 over the seat and
            // 0.05 back, looking at the fire 0.18 up.
            Kind::Campfire => rig(0.0, 1.65, 0.8, 0.0, 0.0, 0.18, 0.0, 1.6),
            // ShelfStation.lean: 1.22 out from the front face, 0.35 up, looking at its middle.
            Kind::Shelf => rig(0.5, 0.5 + 1.22, 0.35, 0.0, 0.5, -0.02, 0.0, 1.45),
        }
    }
}

/// The station kind of a block (`low`: the player's feet are above the block, for the furnaces).
pub fn kind_of(block: &str, low: bool) -> Option<Kind> {
    let _ = low;
    Some(match block {
        "minecraft:crafting_table" => Kind::Table,
        "minecraft:furnace" => Kind::Furnace,
        "minecraft:smoker" => Kind::Smoker,
        "minecraft:blast_furnace" => Kind::BlastFurnace,
        "minecraft:anvil" | "minecraft:chipped_anvil" | "minecraft:damaged_anvil" => Kind::Anvil,
        "minecraft:grindstone" => Kind::Grindstone,
        "minecraft:smithing_table" => Kind::Smithing,
        "minecraft:stonecutter" => Kind::Stonecutter,
        "minecraft:loom" => Kind::Loom,
        "minecraft:cartography_table" => Kind::Cartography,
        "minecraft:enchanting_table" => Kind::Enchanting,
        "minecraft:brewing_stand" => Kind::Brewing,
        "minecraft:lectern" => Kind::Lectern,
        "minecraft:jukebox" => Kind::Jukebox,
        "minecraft:crafter" => Kind::Crafter,
        "minecraft:beacon" => Kind::Beacon,
        "minecraft:chiseled_bookshelf" => Kind::Shelf,
        _ => return None,
    })
}

/// The camera of a station: the eye over its block, looking at the point the jar aims at.
pub fn pose_for(kind: Kind, t: [i32; 3], yaw: f32) -> Pose {
    let r = kind.rig();
    let base = DVec3::new(f64::from(t[0]) + 0.5, f64::from(t[1]) + r.top, f64::from(t[2]) + 0.5);
    let eye = base - forward(yaw) * r.back + right(yaw) * r.lat + DVec3::Y * r.up;
    let look = base - forward(yaw) * r.look_back + right(yaw) * r.look_lat + DVec3::Y * r.look_up;
    // The camera looks at the point (CameraRig.Pose from the look vector: yaw and pitch).
    let d = look - eye;
    let flat = (d.x * d.x + d.z * d.z).sqrt().max(1e-3);
    let turned = if r.lat == r.look_lat { yaw } else { ((-d.x).atan2(d.z).to_degrees() as f32).rem_euclid(360.0) };
    (eye, turned, (-d.y).atan2(flat).to_degrees() as f32)
}

/// How far behind the table's middle and how high above its top the eye is, and its pitch.
pub const BACK: f64 = 0.86;
pub const UP: f64 = 0.72;
/// atan2(0.72, 0.9) in degrees.
pub const PITCH: f32 = 38.66;
/// The scene's field of view: a little zoomed in, so the table fills the screen.
pub const FOV: f32 = 66.0;
/// The grid's cells on the table top: their spacing and half of it.
pub const CELL: f64 = 0.1875;
pub const CELL_HALF: f64 = 0.09375;

pub fn snap_yaw(yaw: f32) -> f32 {
    let s = (yaw / 90.0).round() * 90.0;
    s.rem_euclid(360.0)
}

/// Facing `yaw`: forward and right, flat.
pub fn forward(yaw: f32) -> DVec3 {
    let r = f64::from(yaw).to_radians();
    DVec3::new(-r.sin(), 0.0, r.cos())
}

pub fn right(yaw: f32) -> DVec3 {
    let f = forward(yaw);
    DVec3::new(-f.z, 0.0, f.x)
}

pub fn top_center(t: [i32; 3]) -> DVec3 {
    DVec3::new(f64::from(t[0]) + 0.5, f64::from(t[1]) + 1.0, f64::from(t[2]) + 0.5)
}

/// The camera at the table.
pub fn scene_pose(t: [i32; 3], yaw: f32) -> Pose {
    pose_for(Kind::Table, t, yaw)
}

/// The middle of grid cell `i` (row by row from the far left one) on the table top.
pub fn cell_center(t: [i32; 3], yaw: f32, i: usize) -> DVec3 {
    let (r, c) = ((i / 3) as f64, (i % 3) as f64);
    top_center(t) + right(yaw) * ((c - 1.0) * CELL) + forward(yaw) * ((1.0 - r) * CELL)
}

/// Where the crafted item floats: above the far edge of the table, bobbing.
pub fn result_spot(t: [i32; 3], yaw: f32, seconds: f64) -> DVec3 {
    top_center(t) + forward(yaw) * 0.40 + DVec3::Y * (0.17 + 0.02 * (seconds * 2.2).sin())
}

/// The yaw that looks from the player at the table (the view's own when standing on it).
pub fn facing(player: Option<DVec3>, t: [i32; 3], view_yaw: f32) -> f32 {
    let Some(p) = player else { return snap_yaw(view_yaw) };
    let c = top_center(t);
    let (dx, dz) = (c.x - p.x, c.z - p.z);
    if dx * dx + dz * dz < 0.09 {
        return snap_yaw(view_yaw);
    }
    snap_yaw((-dx).atan2(dz).to_degrees() as f32)
}

/// Ease in and out (cubic).
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

fn angle_lerp(a: f32, b: f32, k: f32) -> f32 {
    let d = (b - a + 540.0).rem_euclid(360.0) - 180.0;
    a + d * k
}

pub fn lerp_pose(a: Pose, b: Pose, k: f32) -> Pose {
    (a.0.lerp(b.0, f64::from(k)), angle_lerp(a.1, b.1, k), a.2 + (b.2 - a.2) * k)
}

/// A crafting table the player clicked (`plain`: a sneak click, which opens the normal screen).
pub fn clicked(t: [i32; 3], plain: bool, now: Instant) {
    clicked_kind(Kind::Table, t, plain, now);
}

/// The same for any station.
pub fn clicked_kind(kind: Kind, t: [i32; 3], plain: bool, now: Instant) {
    let mut s = state();
    s.clicked = Some((t, now, plain));
    s.clicked_kind = kind;
    s.clicked_yaw = None;
}

/// The yaw the station of the last click is worked from (a furnace's front).
pub fn click_yaw(yaw: Option<f32>) {
    state().clicked_yaw = yaw;
}

/// The yaw that looks at a block's front, from its `facing` property.
pub fn yaw_facing(facing: &str) -> Option<f32> {
    Some(match facing {
        "north" => 0.0,
        "south" => 180.0,
        "east" => 90.0,
        "west" => 270.0,
        _ => return None,
    })
}

pub fn kind() -> Kind {
    state().kind
}

/// The recipe book button: this screen goes on as the normal one.
pub fn go_plain() {
    state().plain = true;
}

/// Once a frame, from the camera: whether the crafting screen is open (`fallback` is the table
/// the game itself knows, in a world without a server thread), the game's own camera and the
/// player. The pose to show instead, if any.
pub fn camera(open: bool, fallback: Option<[i32; 3]>, live: Option<Pose>, player: Option<DVec3>, view_yaw: f32, now: Instant) -> Option<Pose> {
    let mut s = state();
    if !s.active {
        s.phase = Phase::Idle;
        return None;
    }
    let (enter, exit) = durations(&s.config);
    let inside = matches!(s.phase, Phase::Enter | Phase::Inside);
    let target = if !open {
        s.plain = false;
        None
    } else if inside {
        (!s.plain).then_some(s.table)
    } else if s.plain {
        None
    } else {
        // A screen that just opened: the table clicked a moment ago, else the game's own.
        let click = s.clicked.take().filter(|c| now.duration_since(c.1).as_secs_f32() < 3.0);
        s.kind = if click.is_some() { s.clicked_kind } else { Kind::Table };
        if click.is_none() {
            s.clicked_yaw = None;
        }
        match click {
            Some((_, _, true)) => {
                s.plain = true;
                None
            }
            Some((t, _, false)) => Some(t),
            None => fallback,
        }
    };
    match (s.phase, target) {
        (Phase::Idle | Phase::Exit, Some(t)) => {
            let fresh = s.phase == Phase::Idle;
            s.from = if fresh { live } else { s.last };
            s.table = t;
            s.yaw = s.clicked_yaw.take().unwrap_or_else(|| facing(player, t, view_yaw));
            s.phase = Phase::Enter;
            s.since = Some(now);
            s.hands = [None, None];
            s.selected = None;
            s.walk = None;
            if fresh && let (Some(l), Some(p)) = (live, player) {
                let stand = stand_for(s.kind, t, s.yaw, p.y);
                let far = DVec3::new(stand.x - p.x, 0.0, stand.z - p.z).length();
                if far > 0.35 {
                    let eye = l.0 - p;
                    let to = (stand + eye, l.1, l.2);
                    s.walk = Some((l, to, (far / WALK_SPEED) as f32 + WALK_LEAD));
                    s.arrived = Some((stand, s.yaw));
                }
            }
        }
        (Phase::Enter | Phase::Inside, Some(t)) if t != s.table => {
            s.from = s.last;
            s.table = t;
            s.yaw = s.clicked_yaw.take().unwrap_or_else(|| facing(player, t, view_yaw));
            s.phase = Phase::Enter;
            s.since = Some(now);
        }
        (Phase::Enter | Phase::Inside, None) => {
            s.from = s.last;
            s.phase = Phase::Exit;
            s.since = Some(now);
        }
        _ => {}
    }
    let k = s.since.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
    let scene = pose_for(s.kind, s.table, s.yaw);
    let pose = match s.phase {
        Phase::Idle => None,
        Phase::Enter => match s.walk {
            Some((from, to, secs)) if k < secs => Some(walk_pose(from, to, k / secs, secs)),
            walk => {
                let start = walk.map(|w| w.1).or(s.from);
                let q = (k - walk.map_or(0.0, |w| w.2)) / enter;
                if q >= 1.0 {
                    s.phase = Phase::Inside;
                }
                let ws = if walk.is_some() { WALK_SHARE } else { 0.0 };
                Some(start.map_or(scene, |f| lean_pose(f, scene, q, ws, 0.045)))
            }
        },
        Phase::Inside => Some(scene),
        Phase::Exit => {
            let k = k / exit;
            match (live, s.from) {
                _ if k >= 1.0 => {
                    s.phase = Phase::Idle;
                    None
                }
                (Some(l), Some(f)) => Some(outro_pose(f, l, k)),
                _ => {
                    s.phase = Phase::Idle;
                    None
                }
            }
        }
    };
    s.last = pose;
    if s.phase == Phase::Idle {
        s.lay = None;
    }
    pose
}

pub fn phase() -> Phase {
    state().phase
}

/// Walking speed to the table (blocks a second) and the pitch the walk ends at.
pub const WALK_SPEED: f64 = 3.6;
/// A beat before the first step.
pub const WALK_LEAD: f32 = 0.25;
pub const WALK_PITCH: f32 = 30.0;

/// Where the player stands at the table: a step in front of it, at their own height.
pub fn stand_spot(t: [i32; 3], yaw: f32, y: f64) -> DVec3 {
    stand_for(Kind::Table, t, yaw, y)
}

pub fn stand_for(kind: Kind, t: [i32; 3], yaw: f32, y: f64) -> DVec3 {
    let c = top_center(t) - forward(yaw) * kind.rig().stand;
    DVec3::new(c.x, y, c.z)
}

/// The eye on the walk at `k` (0..1) of `secs`: steady steps with the view's bob, the head
/// turning to the table early.
pub fn walk_pose(from: Pose, to: Pose, k: f32, secs: f32) -> Pose {
    let k = k.clamp(0.0, 1.0);
    // A beat standing, then a steady pace.
    let along = window(k * secs, WALK_LEAD, secs);
    let mut eye = from.0.lerp(to.0, f64::from(along));
    let walked = from.0.distance(to.0) * f64::from(along);
    // bobView: a step every 0.6 blocks, the eye dips between steps.
    let fade = (along * 8.0).min((1.0 - along) * 8.0).min(1.0);
    let phase = walked / 0.6 * std::f64::consts::PI;
    eye.y -= phase.sin().abs() * 0.045 * f64::from(fade);
    eye += right(to.1) * (phase.cos() * 0.02 * f64::from(fade));
    (eye, from.1, from.2)
}

/// The lean over the table: the eye moves (with a small lift) and turns a little ahead of it.
/// `q` is 0..1 over the lean, `ws` the walk's share of the whole intro.
pub fn lean_pose(start: Pose, scene: Pose, q: f32, ws: f32, lift: f64) -> Pose {
    let q = q.clamp(0.0, 1.0);
    let pos = ease(q);
    let turn = ease(window(ws + (1.0 - ws) * q, ws, 0.82));
    let mut eye = start.0.lerp(scene.0, f64::from(pos));
    eye.y += f64::from(hump(pos)) * lift;
    (eye, angle_lerp(start.1, scene.1, turn), start.2 + (scene.2 - start.2) * turn)
}

/// The way back: the eye leaves the table at 14% of the time, the head at 20%.
pub fn outro_pose(from: Pose, live: Pose, tau: f32) -> Pose {
    let pos = ease(window(tau, 0.14, 1.0));
    let turn = ease(window(tau, 0.2, 1.0));
    let mut eye = from.0.lerp(live.0, f64::from(pos));
    eye.y += f64::from(hump(pos)) * 0.03;
    (eye, angle_lerp(from.1, live.1, turn), from.2 + (live.2 - from.2) * turn)
}

fn walk_secs(s: &State, k: f32) -> f32 {
    match (s.phase, s.walk) {
        (Phase::Enter, Some(w)) => { let _ = k; w.2 }
        _ => 0.0,
    }
}

/// The player walks to the table on the way: still walking.
pub fn walking(now: Instant) -> bool {
    let s = state();
    let k = s.since.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
    s.phase == Phase::Enter && s.walk.is_some_and(|w| k < w.2)
}

/// Once the walk is done: where the player now stands and faces (taken once).
pub fn take_arrival(now: Instant) -> Option<(DVec3, f32)> {
    if walking(now) {
        return None;
    }
    let mut s = state();
    if !matches!(s.phase, Phase::Enter | Phase::Inside) {
        s.arrived = None;
        return None;
    }
    s.arrived.take()
}

/// The scene owns the screen: its drawing and what is under the cursor.
pub fn controls() -> bool {
    let s = state();
    s.active && !s.plain && (s.kind == Kind::Table || is_furnace(s.kind) || is_surface(s.kind)) && matches!(s.phase, Phase::Enter | Phase::Inside)
}

/// The crafting table is on: the arms and the items on its top.
pub fn table_scene() -> bool {
    state().kind == Kind::Table
}

/// The scene is on screen (the camera is at the table or on its way).
pub fn shown() -> bool {
    let s = state();
    s.active && s.phase != Phase::Idle && s.last.is_some()
}

pub fn table() -> ([i32; 3], f32) {
    let s = state();
    (s.table, s.yaw)
}

/// The pose shown this frame.
pub fn view_pose() -> Option<Pose> {
    state().last
}

/// How far the scene is in: 0 at the player, 1 at the table.
pub fn presence(now: Instant) -> f32 {
    let s = state();
    let (enter, exit) = durations(&s.config);
    let k = s.since.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
    let k = k - walk_secs(&s, k);
    match s.phase {
        Phase::Idle => 0.0,
        Phase::Enter => ease(k / enter),
        Phase::Inside => 1.0,
        Phase::Exit => 1.0 - ease(window(k / exit, 0.14, 1.0)),
    }
}

/// The field of view: the player's own, zooming to the scene's at the table.
pub fn scene_fov(live: f32, now: Instant) -> f32 {
    live + (FOV - live) * presence(now)
}

/// How tall the black bars are, as a share of their full height: in over the first 30% of the
/// intro (the walk), out over its last 28%; the same on the way back.
pub fn bars(now: Instant) -> f32 {
    let s = state();
    if !s.config.bars {
        return 0.0;
    }
    let (enter, exit) = durations(&s.config);
    let k = s.since.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
    match s.phase {
        Phase::Enter => {
            let p = match s.walk {
                Some((_, _, secs)) if k < secs => WALK_SHARE * (k / secs.max(0.2)).min(0.98),
                Some((_, _, secs)) => WALK_SHARE + (1.0 - WALK_SHARE) * ((k - secs) / enter).clamp(0.0, 1.0),
                None => (k / enter).clamp(0.0, 1.0),
            };
            sine(window(p, 0.0, 0.3)) * (1.0 - sine(window(p, 0.72, 1.0)))
        }
        Phase::Exit => {
            let tau = k / exit;
            sine(window(tau, 0.0, 0.25)) * (1.0 - sine(window(tau, 0.7, 1.0)))
        }
        _ => 0.0,
    }
}

// ------------------------------------------------------------------ the cursor in the world

pub fn set_lens(fov: f32, aspect: f32) {
    state().lens = (fov, aspect);
}

pub fn set_result_shown(on: bool) {
    state().result_shown = on;
}

pub fn result_shown() -> bool {
    state().result_shown
}

/// The cursor (GUI units) as normalised device coordinates.
pub fn ndc(cursor: (f32, f32), w: f32, h: f32) -> (f32, f32) {
    (cursor.0 / w.max(1.0) * 2.0 - 1.0, 1.0 - cursor.1 / h.max(1.0) * 2.0)
}

/// The view matrix of a pose (Horizon's own).
pub fn view_of(pose: Pose) -> Mat4 {
    crate::camera::view_matrix(pose.1, pose.2)
}

/// The ray through the screen point `n` from the eye of `pose`.
pub fn ray(pose: Pose, n: (f32, f32), fov: f32, aspect: f32) -> DVec3 {
    let t = (fov * 0.5).to_radians().tan();
    let d = view_of(pose).inverse().transform_vector3(Vec3::new(n.0 * t * aspect, n.1 * t, -1.0));
    d.normalize().as_dvec3()
}

/// Where world point `p` is on screen.
pub fn project(pose: Pose, p: DVec3, fov: f32, aspect: f32) -> Option<(f32, f32)> {
    let v = view_of(pose).transform_point3((p - pose.0).as_vec3());
    if v.z > -0.01 {
        return None;
    }
    let t = (fov * 0.5).to_radians().tan();
    Some((v.x / (-v.z * t * aspect), v.y / (-v.z * t)))
}

/// What the screen point `n` points at, with the pose and lens given.
pub fn hover_with(pose: Pose, t: [i32; 3], yaw: f32, n: (f32, f32), fov: f32, aspect: f32, result: bool, seconds: f64) -> Option<Hover> {
    if result && let Some(r) = project(pose, result_spot(t, yaw, seconds), fov, aspect) {
        let (dx, dy) = ((r.0 - n.0) * aspect, r.1 - n.1);
        if dx * dx + dy * dy < 0.11 * 0.11 {
            return Some(Hover::Result);
        }
    }
    let d = ray(pose, n, fov, aspect);
    if d.y > -1e-4 {
        return None;
    }
    let top = top_center(t);
    let along = (top.y - pose.0.y) / d.y;
    let hit = pose.0 + d * along;
    let (lx, lf) = ((hit - top).dot(right(yaw)), (hit - top).dot(forward(yaw)));
    if lx.abs() > 0.5 || lf.abs() > 0.5 {
        return None;
    }
    let (c, f) = ((lx / CELL).round(), (lf / CELL).round());
    if c.abs() <= 1.0 && f.abs() <= 1.0 && (lx - c * CELL).abs() <= CELL_HALF && (lf - f * CELL).abs() <= CELL_HALF {
        return Some(Hover::Cell(((1.0 - f) * 3.0 + c + 1.0) as usize));
    }
    Some(Hover::Table(hit))
}

/// What the cursor points at in the scene now (only at the table, not on the way).
pub fn hover(n: (f32, f32), seconds: f64) -> Option<Hover> {
    let s = state();
    if s.phase != Phase::Inside {
        return None;
    }
    let pose = s.last?;
    hover_with(pose, s.table, s.yaw, n, s.lens.0, s.lens.1, s.result_shown, seconds)
}

// ------------------------------------------------------------------ the arms

/// Where the arm of `side` (0 left, 1 right) starts: 0.38 to that side of the eye, 0.46 below it
/// and 0.08 ahead, along the view's own yaw (`Hands.shoulder`).
pub fn shoulder(side: usize, pose: Pose, yaw: f32) -> DVec3 {
    let _ = yaw;
    let f = if side == 0 { -1.0 } else { 1.0 };
    pose.0 + right(pose.1) * (0.38 * f) - DVec3::Y * 0.46 + forward(pose.1) * 0.08
}

/// Where the hand of `side` rests: on the near edge of the table.
pub fn rest(side: usize, t: [i32; 3], yaw: f32) -> DVec3 {
    let f = if side == 0 { -1.0 } else { 1.0 };
    top_center(t) + right(yaw) * (0.24 * f) + DVec3::Y * 0.062 - forward(yaw) * 0.4
}

/// The hand that reaches for what is hovered, and where it stops: a little short of the spot (0.075
/// along the way from the hand), a little above the top (0.045, with a stack in it 0.075).
pub fn reach_for(h: Hover, t: [i32; 3], yaw: f32, seconds: f64) -> (usize, DVec3) {
    let tip = |p: DVec3| p + DVec3::Y * 0.062 - forward(yaw) * 0.075;
    match h {
        Hover::Cell(i) => (usize::from(i % 3 != 0), tip(cell_center(t, yaw, i))),
        Hover::Result => (1, result_spot(t, yaw, seconds) - forward(yaw) * 0.09 - DVec3::Y * 0.02),
        Hover::Table(p) => {
            let lx = (p - top_center(t)).dot(right(yaw));
            (usize::from(lx >= -CELL_HALF), tip(p))
        }
    }
}

/// A stack in the hand: the hand holds higher over the table.
pub fn set_carrying(on: bool) {
    state().carrying = on;
}

/// One step of a hand on its spring (stiffness, damping) towards `target`.
fn spring(pos: DVec3, speed: &mut DVec3, target: DVec3, k: f64, c: f64, dt: f64) -> DVec3 {
    let a = (target - pos) * k - *speed * c;
    *speed += a * dt;
    pos + *speed * dt
}

/// Moves the hands towards their targets; returns, per hand, its shoulder and its tip. The right
/// hand comes in first on the way in (8% to 85% of the intro), the left later (42% to 93%).
pub fn step_hands(hover: Option<Hover>, now: Instant, seconds: f64) -> Option<[[DVec3; 2]; 2]> {
    let (t, yaw, carrying) = {
        let s = state();
        (s.table, s.yaw, s.carrying)
    };
    let lift = if carrying { 0.03 } else { 0.0 };
    let sway = [
        DVec3::new(0.0, 0.003 * (seconds * 1.7).sin(), 0.0),
        DVec3::new(0.0, 0.003 * (seconds * 1.5 + 2.0).sin(), 0.0),
    ];
    let mut targets = [rest(0, t, yaw) + sway[0], rest(1, t, yaw) + sway[1]];
    let mut reaching = None;
    if let Some(h) = hover {
        let (side, at) = reach_for(h, t, yaw, seconds);
        targets[side] = at + DVec3::Y * lift;
        reaching = Some(side);
    }
    step_hands_to(targets, reaching, now)
}

/// Moves both hands on their springs towards `targets` (`reaching`: the one that works).
pub fn step_hands_to(targets: [DVec3; 2], reaching: Option<usize>, now: Instant) -> Option<[[DVec3; 2]; 2]> {
    let presence = presence(now);
    let mut s = state();
    let pose = s.last?;
    let yaw = s.yaw;
    s.reaching = reaching;
    let dt = f64::from(s.stepped.map_or(1.0, |p| now.duration_since(p).as_secs_f32()).min(0.1));
    s.stepped = Some(now);
    let arrive = [window(presence, 0.42, 0.93), window(presence, 0.08, 0.85)];
    let mut out = [[DVec3::ZERO; 2]; 2];
    for side in 0..2 {
        let (k, c) = if s.reaching == Some(side) { (20.0 * 1.6, 2.0 * (20.0f64 * 1.6).sqrt()) } else if side == 1 { (12.0 * 1.6, 17.0) } else { (17.0 * 1.6, 12.0) };
        let hand = match s.hands[side] {
            Some(h) => {
                let mut v = s.speed[side];
                let n = spring(h, &mut v, targets[side], k, c, dt);
                s.speed[side] = v;
                n
            }
            None => targets[side],
        };
        s.hands[side] = Some(hand);
        // On the way in the hand rises from below the screen.
        let rise = 1.0 - f64::from(ease(arrive[side]));
        let from = shoulder(side, pose, yaw);
        out[side] = [from, hand - DVec3::Y * (0.75 * rise)];
    }
    Some(out)
}

/// A face of the arm: its corners in the world, their skin pixels (0..64) and a shade.
pub struct ArmQuad {
    pub corners: [DVec3; 4],
    pub uv: [[f32; 2]; 4],
    pub shade: f32,
}

/// The arm's model is drawn at 0.55 of its size (`Hands.drawArm`).
pub const ARM_SCALE: f64 = 0.55;

/// Where the arm lies: the model's origin, its axes (y from the shoulder to the hand, -z up) and
/// how much its upper part is stretched so that the arm reaches from 0.18 behind the shoulder.
#[derive(Clone, Copy, Debug)]
pub struct ArmFrame {
    pub origin: DVec3,
    pub x: DVec3,
    pub y: DVec3,
    pub z: DVec3,
    pub stretch: f64,
}

pub fn arm_frame(shoulder: DVec3, hand: DVec3) -> ArmFrame {
    let d = hand - shoulder;
    let len = d.length() + 0.18;
    let y = if d.length_squared() < 1e-9 { DVec3::NEG_Y } else { d.normalize() };
    let mut a = DVec3::Y - y * y.y;
    if a.length_squared() < 1e-6 {
        a = DVec3::Z;
    }
    let z = -a.normalize();
    let x = y.cross(z);
    // ArmMesh: the hand's 4 pixels keep their size, the upper 8 stretch.
    let stretch = ((len * 16.0 / ARM_SCALE - 4.0) / 8.0).max(0.6);
    ArmFrame { origin: hand - y * (10.0 * ARM_SCALE / 16.0), x, y, z, stretch }
}

/// A point of the arm model, in its pixels: y runs from the shoulder (-2) to the hand (10); below
/// 6 it is stretched when `upper`.
pub fn arm_point(f: &ArmFrame, x: f64, y: f64, z: f64, upper: bool) -> DVec3 {
    let y = if upper { 6.0 - (6.0 - y) * f.stretch } else { y };
    f.origin + (f.x * x + f.y * y + f.z * z) * (ARM_SCALE / 16.0)
}

/// Minecraft's face shading: tops full, sides darker, bottoms darkest.
fn shade_of(n: DVec3) -> f32 {
    let up = n.y.max(0.0);
    let down = (-n.y).max(0.0);
    (0.8 + 0.2 * up - 0.3 * down - 0.2 * n.x.abs() * (1.0 - n.y.abs())).clamp(0.45, 1.0) as f32
}

/// The player's arm as the jar's ArmMesh draws it: an upper part (its sides, stretched), the hand
/// (its sides and its end) and, 0.25 pixels larger, the sleeve over both; textured from the skin.
pub fn arm_quads(shoulder: DVec3, hand: DVec3, left: bool, slim: bool) -> Vec<ArmQuad> {
    let f = arm_frame(shoulder, hand);
    let w = if slim { 3.0f32 } else { 4.0 };
    // The skin's arm and sleeve, and how much larger each is.
    let layers: [(f32, f32, f64); 2] = if left { [(32.0, 48.0, 0.0), (48.0, 48.0, 0.25)] } else { [(40.0, 16.0, 0.0), (40.0, 32.0, 0.25)] };
    let mut out = Vec::new();
    for (u, v, g) in layers {
        let hx = f64::from(w) * 0.5 + g;
        let hz = 2.0 + g;
        // The four sides: their outward normal (model axes), the two corners along the side and
        // the skin columns.
        let sides: [([f64; 3], [f64; 2], [f64; 2], f32, f32); 4] = [
            ([0.0, 0.0, -1.0], [-hx, -hz], [hx, -hz], u + 4.0, u + 4.0 + w),
            ([1.0, 0.0, 0.0], [hx, -hz], [hx, hz], u + 4.0 + w, u + 8.0 + w),
            ([0.0, 0.0, 1.0], [hx, hz], [-hx, hz], u + 8.0 + w, u + 8.0 + 2.0 * w),
            ([-1.0, 0.0, 0.0], [-hx, hz], [-hx, -hz], u, u + 4.0),
        ];
        // The upper part (rows 4..12 of the arm) and the hand (rows 12..16).
        for (y0, y1, r0, r1, upper) in [(-2.0 - g, 6.0 + g, v + 4.0, v + 12.0, true), (6.0 - g, 10.0 + g, v + 12.0, v + 16.0, false)] {
            for (n, p, q, c0, c1) in sides {
                let world_n = f.x * n[0] + f.y * n[1] + f.z * n[2];
                out.push(ArmQuad {
                    corners: [
                        arm_point(&f, p[0], y0, p[1], upper),
                        arm_point(&f, q[0], y0, q[1], upper),
                        arm_point(&f, q[0], y1, q[1], upper),
                        arm_point(&f, p[0], y1, p[1], upper),
                    ],
                    uv: [[c0, r0], [c1, r0], [c1, r1], [c0, r1]],
                    shade: shade_of(world_n),
                });
            }
        }
        // The hand's end.
        let y = 10.0 + g;
        let (c0, c1) = (u + 4.0 + w, u + 4.0 + 2.0 * w);
        out.push(ArmQuad {
            corners: [
                arm_point(&f, -hx, y, -hz, false),
                arm_point(&f, hx, y, -hz, false),
                arm_point(&f, hx, y, hz, false),
                arm_point(&f, -hx, y, hz, false),
            ],
            uv: [[c0, v], [c1, v], [c1, v + 4.0], [c0, v + 4.0]],
            shade: shade_of(f.y),
        });
    }
    out
}

/// Seconds a hand spends at each cell while laying out, and when in it the hand reaches the cell.
pub const LAY_STEP: f32 = 0.3;
pub const LAY_REACH: f32 = 0.22;

/// One step of laying a recipe out: at `cell` the hand takes back what is in the way (`take`), or
/// lays `count` of any of `items` there.
#[derive(Clone, Debug, PartialEq)]
pub struct LayStep {
    pub cell: usize,
    pub take: bool,
    pub items: Vec<String>,
    pub count: u32,
}

/// Begins laying out: the hands go through the plan, a cell at a time.
pub fn start_lay(plan: Vec<LayStep>, now: Instant) {
    let mut s = state();
    if plan.is_empty() {
        s.lay = None;
        return;
    }
    s.lay = Some((now, plan.iter().map(|p| p.cell).collect(), 0, false));
    s.lay_plan = plan;
    s.lay_done = 0;
}

pub fn laying() -> bool {
    state().lay.is_some()
}

/// The cell the hands are at while laying out.
pub fn lay_target(now: Instant) -> Option<usize> {
    let s = state();
    let (at, cells, _, _) = s.lay.as_ref()?;
    let i = (now.duration_since(*at).as_secs_f32() / LAY_STEP) as usize;
    cells.get(i.min(cells.len().saturating_sub(1))).copied()
}

/// The steps the hand has reached since last asked, in order (each once); the plan ends after its
/// last step.
pub fn take_lay_steps(now: Instant) -> Vec<LayStep> {
    let mut s = state();
    let Some((at, _, _, _)) = s.lay.clone() else {
        return Vec::new();
    };
    let t = now.duration_since(at).as_secs_f32();
    let due = (((t - LAY_REACH) / LAY_STEP).floor() as i64 + 1).clamp(0, s.lay_plan.len() as i64) as usize;
    let out = s.lay_plan[s.lay_done.min(due)..due].to_vec();
    s.lay_done = due;
    if due >= s.lay_plan.len() && t >= s.lay_plan.len() as f32 * LAY_STEP {
        s.lay = None;
        s.lay_plan.clear();
        s.lay_done = 0;
    }
    out
}

pub fn cancel_lay() {
    let mut s = state();
    s.lay = None;
    s.lay_plan.clear();
    s.lay_done = 0;
}

// ------------------------------------------------------------------ the recipe list

/// The panel's own filter: only what can be made from the inventory.
pub fn book_filter() -> bool {
    state().book_filter
}

pub fn set_book_filter(on: bool) {
    let mut s = state();
    s.book_filter = on;
    s.book_list.0 = 0;
}

/// The panel's list, if it was built from `key`.
pub fn book_list(key: u64) -> Option<Vec<(usize, bool)>> {
    let s = state();
    (s.book_list.0 == key && key != 0).then(|| s.book_list.1.clone())
}

pub fn set_book_list(key: u64, list: Vec<(usize, bool)>) {
    let mut s = state();
    let pages = list.len().div_ceil(BOOK_PAGE).max(1);
    if s.book_page >= pages {
        s.book_page = 0;
    }
    s.book_list = (key, list);
}

/// The list as last built.
pub fn book_list_current() -> Vec<(usize, bool)> {
    state().book_list.1.clone()
}

pub fn book_page() -> usize {
    state().book_page
}

pub fn set_book_page(page: usize) {
    state().book_page = page;
}

/// How many whole sets of the guide the stock covers: per group of cells wanting the same items,
/// what the stock has of them over how many cells want them.
pub fn lay_sets(wants: &[(usize, Vec<String>)], stock: &[(String, u32)]) -> u32 {
    let mut groups: Vec<(&Vec<String>, u32)> = Vec::new();
    for (_, items) in wants {
        match groups.iter_mut().find(|g| g.0 == items) {
            Some(g) => g.1 += 1,
            None => groups.push((items, 1)),
        }
    }
    groups
        .iter()
        .map(|(items, per)| stock.iter().filter(|s| items.contains(&s.0)).map(|s| s.1).sum::<u32>() / per)
        .min()
        .unwrap_or(0)
}

/// The hands' plan for laying the guide out (`sets` of it): first they take back what is in the
/// way (a wrong or an extra item), then lay each cell the stock has an item for.
pub fn lay_plan(wants: &[(usize, Vec<String>)], grid: &[Option<(String, u32)>], stock: &[(String, u32)], sets: u32) -> Vec<LayStep> {
    let sets = sets.max(1);
    let mut takes = Vec::new();
    let mut puts = Vec::new();
    for c in 0..9 {
        let there = grid.get(c).cloned().flatten();
        let want = wants.iter().find(|w| w.0 == c).map(|w| w.1.clone());
        let take = LayStep { cell: c, take: true, items: Vec::new(), count: 0 };
        match (there, want) {
            (Some(_), None) => takes.push(take),
            (Some((item, _)), Some(w)) if !w.contains(&item) => {
                takes.push(take);
                puts.push(LayStep { cell: c, take: false, items: w, count: sets });
            }
            (Some((item, n)), Some(_)) => {
                if sets > n {
                    puts.push(LayStep { cell: c, take: false, items: vec![item], count: sets - n });
                }
            }
            (None, Some(w)) => puts.push(LayStep { cell: c, take: false, items: w, count: sets }),
            (None, None) => {}
        }
    }
    puts.retain(|p| stock.iter().any(|s| p.items.contains(&s.0)));
    takes.extend(puts);
    takes
}

/// A name without its formatting codes, for sorting.
pub fn plain_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut skip = false;
    for c in name.chars() {
        if skip {
            skip = false;
        } else if c == '\u{a7}' {
            skip = true;
        } else {
            out.push(c);
        }
    }
    out
}

/// TableRecipeGuide's order: by the result's name, then the recipe id.
pub fn sort_recipes(list: &mut [(String, String, usize, bool)]) {
    list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
}

/// Whether a recipe's result name matches the search (any case).
pub fn search_matches(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || name.to_lowercase().contains(&q)
}

/// The hand that reaches now (0 left, 1 right).
pub fn reaching() -> Option<usize> {
    state().reaching
}

/// The scale from the hand pass's field of view to the world's (the hand pass draws over it).
pub fn fov_k(hud_fov: f32, world_fov: f32) -> f32 {
    (hud_fov * 0.5).to_radians().tan() / (world_fov * 0.5).to_radians().tan().max(1e-3)
}

/// A camera-relative point moved so that the hand pass shows it where the world would.
pub fn fov_fix(p: Vec3, view: Mat4, inverse: Mat4, k: f32) -> Vec3 {
    let mut v = view.transform_point3(p);
    v.x *= k;
    v.y *= k;
    inverse.transform_point3(v)
}

/// The arm's matrix: the model's `bounds` (pixels, its shoulder at the low y) from `shoulder`
/// to `hand`, stretched along its length, the back of the hand up.
pub fn arm_matrix(shoulder: Vec3, hand: Vec3, bounds: ([f32; 3], [f32; 3])) -> Mat4 {
    let (min, max) = bounds;
    let d = hand - shoulder;
    let len = d.length().max(0.05);
    let y = d / len;
    let side = y.cross(Vec3::Y);
    let side = if side.length_squared() < 1e-6 { Vec3::X } else { side.normalize() };
    let x = -side;
    let z = x.cross(y);
    let model_len = ((max[1] - min[1]) / 16.0).max(0.01);
    let top = Vec3::new((min[0] + max[0]) * 0.5, min[1], (min[2] + max[2]) * 0.5) / 16.0;
    Mat4::from_cols(x.extend(0.0), (y * (len / model_len)).extend(0.0), z.extend(0.0), shoulder.extend(1.0))
        * Mat4::from_translation(-top)
}

/// An item lying on the table at `at`: a block stands on it, a flat item lies down.
pub fn item_matrix(at: Vec3, yaw: f32, block: bool, size: f32, spin: f32) -> Mat4 {
    let turn = Mat4::from_rotation_y((180.0 - yaw + spin).to_radians());
    if block {
        Mat4::from_translation(at + Vec3::Y * (size * 0.5)) * turn * Mat4::from_scale(Vec3::splat(size * 2.0))
    } else {
        Mat4::from_translation(at + Vec3::Y * 0.012)
            * turn
            * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Mat4::from_scale(Vec3::splat(size * 2.0))
    }
}

/// The highlight around a cell: four thin strips on the table top (camera-relative corners).
pub fn frame_quads(center: DVec3, yaw: f32) -> Vec<[DVec3; 4]> {
    let (r, f) = (right(yaw), forward(yaw));
    let (a, w) = (0.098, 0.011);
    let c = center + DVec3::Y * 0.004;
    let strip = |o: DVec3, half_r: f64, half_f: f64| {
        [
            c + o - r * half_r - f * half_f,
            c + o + r * half_r - f * half_f,
            c + o + r * half_r + f * half_f,
            c + o - r * half_r + f * half_f,
        ]
    };
    vec![
        strip(f * a, a + w, w),
        strip(-f * a, a + w, w),
        strip(r * a, w, a - w),
        strip(-r * a, w, a - w),
    ]
}

// ------------------------------------------------------------------ range

/// Whether the player can reach the table from where they stand: "Too low" (the table is two
/// blocks or more below the feet, or one below and not right beside it), "Too high".
pub fn reach(feet: DVec3, t: [i32; 3]) -> Option<&'static str> {
    let dy = (feet.y + 1e-3).floor() as i32 - t[1];
    let (bx, bz) = (feet.x.floor() as i32, feet.z.floor() as i32);
    let near = (bx - t[0]).abs() <= 1 && (bz - t[2]).abs() <= 1;
    if dy >= 2 || (dy == 1 && !near) {
        Some("Too low")
    } else if dy <= -2 {
        Some("Too high")
    } else {
        None
    }
}

pub fn toast(text: &'static str, now: Instant) {
    state().toast = Some((text, now));
}

// ------------------------------------------------------------------ the screen

pub const PANEL_W: f32 = 161.0;
pub const SLOT: f32 = 17.0;
pub const HINTS: &str = "LMB take / put down (drag to share)  \u{b7}  RMB one at a time (drag to spread)  \u{b7}  Shift quick  \u{b7}  Esc step back";

/// The inventory panel at the top: x, y, w, h.
pub fn panel(w: f32, expanded: bool) -> [f32; 4] {
    [((w - PANEL_W) * 0.5).floor(), 10.0, PANEL_W, if expanded { 80.0 } else { 26.0 }]
}

/// The top left corner of inventory slot `i`'s item in the panel.
pub fn slot_pos(w: f32, i: usize) -> (f32, f32) {
    let [x, y, _, _] = panel(w, true);
    let x0 = x + 4.5;
    if i < 9 {
        (x0 + i as f32 * SLOT, y + 5.0)
    } else {
        let r = (i - 9) / 9;
        (x0 + ((i - 9) % 9) as f32 * SLOT, y + 26.0 + r as f32 * SLOT)
    }
}

fn inside(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}

/// The panel opens while the cursor is over it.
pub fn update_expanded(w: f32, x: f32, y: f32) -> bool {
    let mut s = state();
    let e = inside(panel(w, s.expanded), x, y);
    s.expanded = e;
    e
}

pub fn expanded() -> bool {
    state().expanded
}

pub fn on_panel(w: f32, x: f32, y: f32) -> bool {
    inside(panel(w, expanded()), x, y)
}

/// The recipe book button at the left.
pub fn recipe_button(h: f32) -> [f32; 4] {
    [6.0, (h * 0.466).floor(), 24.0, 20.0]
}

pub fn on_recipe(h: f32, x: f32, y: f32) -> bool {
    inside(recipe_button(h), x, y)
}

/// The panel's slot under the cursor.
pub fn panel_slot(w: f32, x: f32, y: f32, expanded: bool) -> Option<usize> {
    if !inside(panel(w, expanded), x, y) {
        return None;
    }
    (0..if expanded { 36 } else { 9 }).find(|&i| {
        let (sx, sy) = slot_pos(w, i);
        x >= sx - 0.5 && x < sx + 16.5 && y >= sy - 0.5 && y < sy + 16.5
    })
}

/// The menu slot under the cursor in the scene: the panel's inventory slots (0..36), the grid's
/// cells (100..109) and the result (200).
pub fn slot_at(w: f32, h: f32, x: f32, y: f32, seconds: f64) -> Option<usize> {
    let e = expanded();
    if book_open() && inside(book_rect(), x, y) {
        return None;
    }
    if inside(panel(w, e), x, y) {
        return panel_slot(w, x, y, e);
    }
    if on_recipe(h, x, y) {
        return None;
    }
    match hover(ndc((x, y), w, h), seconds)? {
        Hover::Cell(i) => Some(100 + i),
        Hover::Result => Some(200),
        Hover::Table(_) => None,
    }
}

/// Text at `scale`, centred on `cx`.
fn text_scaled(batch: &mut HudBatch, atlas: &HudAtlas, text: &str, cx: f32, y: f32, scale: f32, color: u32) {
    let w = HudBatch::text_width(atlas, text);
    let start = batch.vertices.len();
    batch.text(atlas, text, cx - w * 0.5, y, color, false);
    batch.scale_since(start, cx, y + 4.0, scale, scale);
}

pub fn draw_panel(batch: &mut HudBatch, atlas: &HudAtlas, expanded: bool, selected: usize, hovered: Option<usize>) {
    let [x, y, w, h] = panel(batch.width, expanded);
    batch.fill(atlas, x, y, w, h, rgba(52, 52, 56, 150));
    batch.fill(atlas, x, y, w, 1.0, rgba(120, 120, 126, 140));
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, rgba(24, 24, 26, 150));
    batch.fill(atlas, x, y, 1.0, h, rgba(110, 110, 116, 120));
    batch.fill(atlas, x + w - 1.0, y, 1.0, h, rgba(110, 110, 116, 120));
    let count = if expanded { 36 } else { 9 };
    for i in 0..count {
        let (sx, sy) = slot_pos(batch.width, i);
        batch.fill(atlas, sx - 0.5, sy - 0.5, 16.0, 16.0, rgba(30, 30, 34, 95));
        if hovered == Some(i) {
            batch.fill(atlas, sx - 0.5, sy - 0.5, 16.0, 16.0, rgba(255, 255, 255, 70));
        }
    }
    if expanded {
        let (sx, _) = slot_pos(batch.width, 0);
        batch.fill(atlas, sx, y + 23.5, SLOT * 9.0 - 1.0, 1.0, rgba(150, 150, 156, 110));
    } else {
        batch.fill(atlas, x + w * 0.5 - 8.0, y + h - 3.0, 16.0, 1.0, rgba(170, 170, 176, 150));
    }
    // The selected hotbar slot: a gold frame.
    let (sx, sy) = slot_pos(batch.width, selected.min(8));
    let gold = rgba(232, 204, 132, 230);
    batch.fill(atlas, sx - 1.0, sy - 1.0, 17.0, 1.0, gold);
    batch.fill(atlas, sx - 1.0, sy + 15.0, 17.0, 1.0, gold);
    batch.fill(atlas, sx - 1.0, sy - 1.0, 1.0, 17.0, gold);
    batch.fill(atlas, sx + 15.0, sy - 1.0, 1.0, 17.0, gold);
}

pub fn draw_recipe_button(batch: &mut HudBatch, atlas: &HudAtlas, hover: bool) {
    let [x, y, w, h] = recipe_button(batch.height);
    batch.fill(atlas, x, y, w, h, if hover { rgba(90, 90, 96, 190) } else { rgba(56, 56, 60, 170) });
    batch.fill(atlas, x, y, w, 1.0, rgba(130, 130, 136, 150));
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, rgba(24, 24, 26, 170));
}

pub fn draw_hints(batch: &mut HudBatch, atlas: &HudAtlas) {
    draw_hint_line(batch, atlas, HINTS);
}

/// The result slot's hint, in place of the usual one.
pub const RESULT_HINT: &str = "Click to craft \u{b7} Shift-click to craft all";

/// A line of hint at the bottom, in the hints' style.
pub fn draw_hint_line(batch: &mut HudBatch, atlas: &HudAtlas, text: &str) {
    let wide = HudBatch::text_width(atlas, text).max(1.0);
    let scale = 0.8f32.min((batch.width - 20.0) / wide);
    let w = wide * scale + 14.0;
    let cx = batch.width * 0.5;
    let y = batch.height - 16.0;
    batch.fill(atlas, cx - w * 0.5, y - 2.5, w, 12.0, rgba(16, 16, 28, 90));
    text_scaled(batch, atlas, text, cx, y, scale, rgba(222, 222, 232, 235));
}

/// What a drag over the cells shares out: left button evenly, right one each. Returns the text for
/// "Shared out: A into each of N cells" and "Left in hand: B".
pub fn spread_text(carried: u32, cells: usize, right: bool) -> Option<String> {
    if cells < 2 || carried == 0 {
        return None;
    }
    let n = cells as u32;
    let (each, left) = if right { (1, carried.saturating_sub(n)) } else { (carried / n, carried % n) };
    if each == 0 {
        return None;
    }
    Some(format!("Shared out: {each} into each of {n} cells  \u{b7}  Left in hand: {left}"))
}

/// The black bars and the range message.
pub fn draw_overlay(batch: &mut HudBatch, atlas: &HudAtlas, now: Instant) {
    let b = bars(now);
    if b > 0.0 {
        let h = batch.height * 0.085 * b;
        batch.fill(atlas, 0.0, 0.0, batch.width, h, rgba(0, 0, 0, 255));
        batch.fill(atlas, 0.0, batch.height - h, batch.width, h, rgba(0, 0, 0, 255));
    }
    let toast = state().toast;
    if let Some((text, at)) = toast {
        let age = now.duration_since(at).as_secs_f32();
        if age > 1.8 {
            state().toast = None;
            return;
        }
        let a = if age > 1.4 { 1.0 - (age - 1.4) / 0.4 } else { 1.0 };
        let alpha = |v: f32| (v * a).round().clamp(0.0, 255.0) as u8;
        let scale = 0.8;
        let w = HudBatch::text_width(atlas, text) * scale + 16.0;
        let (cx, y) = (batch.width * 0.5, (batch.height * 0.80).floor());
        let x = cx - w * 0.5;
        batch.fill(atlas, x, y, w, 10.0, rgba(58, 40, 40, alpha(200.0)));
        batch.fill(atlas, x, y, w, 0.5, rgba(150, 96, 86, alpha(220.0)));
        batch.fill(atlas, x, y + 9.5, w, 0.5, rgba(150, 96, 86, alpha(220.0)));
        batch.fill(atlas, x + 3.5, y + 3.5, 2.5, 2.5, rgba(236, 128, 84, alpha(255.0)));
        text_scaled(batch, atlas, text, cx + 3.0, y + 1.5, scale, rgba(240, 232, 228, alpha(255.0)));
    }
}

// ------------------------------------------------------------------ the recipe panel

/// The panel is 128 x 226 in its own units, drawn at `scale` (1 on a big screen, less on a small
/// one) 8 units in from the left and 15% down.
pub const BOOK_W: f32 = 128.0;
pub const BOOK_H: f32 = 226.0;
pub const BOOK_PAGE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BookHit {
    Minimize,
    Search,
    Filter,
    /// A recipe of the page (0..16).
    Recipe(usize),
    Prev,
    Next,
    Clear,
    LayOut,
    /// On the panel, nothing to click.
    Panel,
}

pub fn set_book_open(open: bool) {
    state().book_open = open;
}

pub fn book_open() -> bool {
    state().book_open
}

pub fn selected() -> Option<usize> {
    state().selected
}

pub fn select(entry: Option<usize>) {
    state().selected = entry;
}

/// The screen's size in GUI units, as the last frame drew it.
pub fn set_gui(w: f32, h: f32) {
    state().gui = (w.max(1.0), h.max(1.0));
}

/// The panel's scale: 1, or less when the screen is small.
pub fn book_scale() -> f32 {
    let (w, h) = state().gui;
    (h / 300.0).min(w / 530.0).min(1.0)
}

/// The panel on screen: x, y, w, h.
pub fn book_rect() -> [f32; 4] {
    let (w, h) = state().gui;
    let s = (h / 300.0).min(w / 530.0).min(1.0);
    [(8.0 * s).round(), (h * 0.15).round(), BOOK_W * s, BOOK_H * s]
}

/// A rectangle in the panel's own units, on screen.
fn local(x0: f32, y0: f32, x1: f32, y1: f32) -> [f32; 4] {
    let [bx, by, _, _] = book_rect();
    let s = book_scale();
    [bx + x0 * s, by + y0 * s, (x1 - x0) * s, (y1 - y0) * s]
}

/// The top left of the page's recipe `k`: a 16 unit icon in a 27 unit cell, four across.
pub fn book_icon(k: usize) -> (f32, f32) {
    let r = local(10.0 + (k % 4) as f32 * 27.0 + 5.5, 61.0 + (k / 4) as f32 * 27.0 + 5.5, 0.0, 0.0);
    (r[0], r[1])
}

pub fn book_search() -> [f32; 4] {
    local(8.0, 25.0, 120.0, 43.0)
}

/// Minimise, the filter, the previous page, the next page, clear and Lay out.
pub fn book_buttons() -> [[f32; 4]; 6] {
    [
        local(103.0, 0.0, 128.0, 23.0),
        local(8.0, 46.0, 120.0, 59.0),
        local(0.0, 169.0, 32.0, 184.0),
        local(96.0, 169.0, 128.0, 184.0),
        local(103.0, 186.0, 128.0, 205.0),
        local(8.0, 205.0, 120.0, 220.0),
    ]
}

/// What the cursor is on in the open panel.
pub fn book_hit(x: f32, y: f32, has_choice: bool) -> Option<BookHit> {
    if !inside(book_rect(), x, y) {
        return None;
    }
    let b = book_buttons();
    if inside(b[0], x, y) {
        return Some(BookHit::Minimize);
    }
    if inside(book_search(), x, y) {
        return Some(BookHit::Search);
    }
    if inside(b[1], x, y) {
        return Some(BookHit::Filter);
    }
    if inside(b[2], x, y) {
        return Some(BookHit::Prev);
    }
    if inside(b[3], x, y) {
        return Some(BookHit::Next);
    }
    if has_choice && inside(b[4], x, y) {
        return Some(BookHit::Clear);
    }
    if has_choice && inside(b[5], x, y) {
        return Some(BookHit::LayOut);
    }
    let [bx, by, _, _] = book_rect();
    let s = book_scale();
    let (lx, ly) = ((x - bx) / s - 10.0, (y - by) / s - 61.0);
    if (0.0..108.0).contains(&lx) && (0.0..108.0).contains(&ly) {
        return Some(BookHit::Recipe((ly / 27.0) as usize * 4 + (lx / 27.0) as usize));
    }
    Some(BookHit::Panel)
}

/// Text at `scale`, its top left at `x`, `y`.
pub fn scaled_text(batch: &mut HudBatch, atlas: &HudAtlas, text: &str, x: f32, y: f32, scale: f32, color: u32) {
    let start = batch.vertices.len();
    batch.text(atlas, text, x, y, color, false);
    batch.scale_since(start, x, y, scale, scale);
}

fn rim(batch: &mut HudBatch, atlas: &HudAtlas, r: [f32; 4], fill: u32, edge: u32) {
    batch.fill(atlas, r[0], r[1], r[2], r[3], fill);
    batch.fill(atlas, r[0], r[1], r[2], 1.0, edge);
    batch.fill(atlas, r[0], r[1] + r[3] - 1.0, r[2], 1.0, edge);
    batch.fill(atlas, r[0], r[1], 1.0, r[3], edge);
    batch.fill(atlas, r[0] + r[2] - 1.0, r[1], 1.0, r[3], edge);
}

/// The panel's glass, title, search box, filter, page line and footer (the icons are the game's).
/// `lay` is the text of the Lay out button and whether it can be pressed.
pub fn draw_book(batch: &mut HudBatch, atlas: &HudAtlas, search: &str, focused: bool, filtering: bool, page: usize, pages: usize, choice: Option<&str>, hover: Option<BookHit>, blink: bool, lay: (&str, bool)) {
    let r = book_rect();
    let s = book_scale();
    let light = rgba(236, 236, 240, 255);
    let grey = rgba(160, 160, 168, 255);
    rim(batch, atlas, r, rgba(26, 28, 34, 190), rgba(120, 124, 136, 190));
    let at = |x: f32, y: f32| (r[0] + x * s, r[1] + y * s);
    let (tx, ty) = at(8.0, 8.0);
    scaled_text(batch, atlas, "Recipes", tx, ty, s, light);
    let b = book_buttons();
    let minus = if hover == Some(BookHit::Minimize) { light } else { grey };
    batch.fill(atlas, b[0][0] + 8.0 * s, b[0][1] + 11.0 * s, 9.0 * s, s.max(1.0), minus);
    let sr = book_search();
    rim(batch, atlas, sr, if focused { rgba(10, 10, 14, 230) } else { rgba(18, 18, 24, 200) }, rgba(90, 94, 106, 200));
    if search.is_empty() && !focused {
        scaled_text(batch, atlas, "Search recipes\u{2026}", sr[0] + 4.0 * s, sr[1] + 5.0 * s, s, grey);
    } else {
        let shown = if focused && blink { format!("{search}_") } else { search.to_owned() };
        scaled_text(batch, atlas, &shown, sr[0] + 4.0 * s, sr[1] + 5.0 * s, s, light);
    }
    // Have materials: a box, ticked when filtering.
    let fr = local(8.0, 47.0, 17.0, 56.0);
    rim(batch, atlas, fr, rgba(18, 18, 24, 220), rgba(110, 114, 126, 220));
    if filtering {
        batch.fill(atlas, fr[0] + 2.0 * s, fr[1] + 2.0 * s, 5.0 * s, 5.0 * s, rgba(120, 220, 120, 255));
    }
    scaled_text(batch, atlas, "Have materials", fr[0] + 14.0 * s, fr[1] + s, s, if hover == Some(BookHit::Filter) { light } else { grey });
    for k in 0..BOOK_PAGE {
        let (ix, iy) = book_icon(k);
        let cell = local(10.0 + (k % 4) as f32 * 27.0, 61.0 + (k / 4) as f32 * 27.0, 37.0 + (k % 4) as f32 * 27.0, 88.0 + (k / 4) as f32 * 27.0);
        batch.fill(atlas, cell[0] + s, cell[1] + s, cell[2] - 2.0 * s, cell[3] - 2.0 * s, rgba(10, 10, 14, 120));
        if hover == Some(BookHit::Recipe(k)) {
            batch.fill(atlas, ix - 2.0 * s, iy - 2.0 * s, 20.0 * s, 20.0 * s, rgba(255, 255, 255, 60));
        }
    }
    let line = format!("{} / {}", page + 1, pages.max(1));
    let lw = HudBatch::text_width(atlas, &line) * s;
    let (_, py) = at(0.0, 173.0);
    scaled_text(batch, atlas, &line, r[0] + r[2] * 0.5 - lw * 0.5, py, s, grey);
    scaled_text(batch, atlas, "<", b[2][0] + 10.0 * s, py, s, if hover == Some(BookHit::Prev) { light } else { grey });
    scaled_text(batch, atlas, ">", b[3][0] + 18.0 * s, py, s, if hover == Some(BookHit::Next) { light } else { grey });
    batch.fill(atlas, r[0] + 8.0 * s, r[1] + 185.0 * s, 112.0 * s, 1.0, rgba(110, 114, 126, 160));
    match choice {
        None => scaled_text(batch, atlas, "Choose a recipe", r[0] + 8.0 * s, r[1] + 191.0 * s, s, grey),
        Some(name) => {
            let mut name = name.to_owned();
            while HudBatch::text_width(atlas, &name) * s > 92.0 * s && name.pop().is_some() {}
            scaled_text(batch, atlas, &name, r[0] + 8.0 * s, r[1] + 191.0 * s, s, light);
            scaled_text(batch, atlas, "\u{d7}", b[4][0] + 9.0 * s, b[4][1] + 6.0 * s, s, if hover == Some(BookHit::Clear) { light } else { grey });
            let (text, ready) = lay;
            let color = if ready { if hover == Some(BookHit::LayOut) { light } else { rgba(210, 214, 222, 255) } } else { rgba(110, 112, 120, 255) };
            rim(batch, atlas, b[5], if ready && hover == Some(BookHit::LayOut) { rgba(70, 90, 70, 220) } else { rgba(36, 40, 48, 220) }, rgba(110, 114, 126, 200));
            let tw = HudBatch::text_width(atlas, text) * s;
            scaled_text(batch, atlas, text, b[5][0] + b[5][2] * 0.5 - tw * 0.5, b[5][1] + 4.0 * s, s, color);
        }
    }
}

// ------------------------------------------------------------------ the furnaces

/// Which face of a furnace a zone or bed is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Front,
    Top,
}

/// A place to work in a furnace (FurnaceLayout.Zone): its slot (0 to smelt, 1 fuel, 2 the
/// result), the rectangle on its face the cursor finds it in, and the one lit up around it.
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub slot: usize,
    pub face: Face,
    pub u0: f64,
    pub u1: f64,
    pub v0: f64,
    pub v1: f64,
    pub hl: [f64; 4],
}

/// Where a slot's items lie (FurnaceLayout.Bed): across u0..u1 at `floor`, `depth` inside the face,
/// up to `copies` of them side by side.
#[derive(Clone, Copy, Debug)]
pub struct Bed {
    pub slot: usize,
    pub face: Face,
    pub u0: f64,
    pub u1: f64,
    pub floor: f64,
    pub depth: f64,
    pub copies: u32,
    pub tilt: f32,
}

pub struct FurnaceLayout {
    pub zones: [Zone; 3],
    pub beds: [Bed; 3],
}

const fn zone(slot: usize, face: Face, u0: f64, u1: f64, v0: f64, v1: f64, hl: [f64; 4]) -> Zone {
    Zone { slot, face, u0, u1, v0, v1, hl }
}

const fn bed(slot: usize, face: Face, u0: f64, u1: f64, floor: f64, depth: f64, copies: u32, tilt: f32) -> Bed {
    Bed { slot, face, u0, u1, floor, depth, copies, tilt }
}

const PX: f64 = 1.0 / 16.0;

/// FurnaceLayout.FURNACE, SMOKER and BLAST_FURNACE.
pub const FURNACE: FurnaceLayout = FurnaceLayout {
    zones: [
        zone(0, Face::Front, 0.11, 0.5, 0.5, 0.92, [3.0 * PX, 0.5, 9.0 * PX, 13.0 * PX]),
        zone(2, Face::Front, 0.5, 0.89, 0.5, 0.92, [0.5, 13.0 * PX, 9.0 * PX, 13.0 * PX]),
        zone(1, Face::Front, 0.11, 0.89, 0.0, 0.39, [3.0 * PX, 13.0 * PX, PX, 5.0 * PX]),
    ],
    beds: [
        bed(0, Face::Front, 0.22, 0.47, 9.0 * PX, 0.035, 3, 0.0),
        bed(2, Face::Front, 0.53, 0.78, 9.0 * PX, 0.035, 3, 0.0),
        bed(1, Face::Front, 0.26, 0.74, PX, 0.035, 4, 0.0),
    ],
};

pub const SMOKER: FurnaceLayout = FurnaceLayout {
    zones: [
        zone(0, Face::Front, 0.1, 0.47, 0.46, 0.86, [3.0 * PX, 7.5 * PX, 0.5, 0.75]),
        zone(2, Face::Front, 0.47, 0.84, 0.46, 0.86, [7.5 * PX, 0.75, 0.5, 0.75]),
        zone(1, Face::Front, 0.08, 0.92, 0.0, 0.46, [2.0 * PX, 14.0 * PX, PX, 7.0 * PX]),
    ],
    beds: [
        bed(0, Face::Front, 0.24, 0.43, 0.5, 0.035, 3, 0.0),
        bed(2, Face::Front, 0.5, 0.7, 0.5, 0.035, 3, 0.0),
        bed(1, Face::Front, 0.22, 0.78, PX, 0.035, 4, 0.0),
    ],
};

pub const BLAST_FURNACE: FurnaceLayout = FurnaceLayout {
    zones: [
        zone(0, Face::Top, 0.06, 0.94, 0.12, 0.94, [0.22, 0.78, 0.3, 0.86]),
        zone(2, Face::Front, 0.12, 0.88, 0.52, 0.86, [3.0 * PX, 13.0 * PX, 9.0 * PX, 10.5 * PX]),
        zone(1, Face::Front, 0.12, 0.88, 0.16, 0.52, [4.0 * PX, 12.0 * PX, 5.0 * PX, 8.0 * PX]),
    ],
    beds: [
        bed(0, Face::Top, 0.32, 0.68, 0.58, 0.045, 3, 62.0),
        bed(2, Face::Front, 0.3, 0.7, 9.0 * PX, 0.035, 3, 0.0),
        bed(1, Face::Front, 0.32, 0.68, 5.0 * PX, 0.035, 3, 0.0),
    ],
};

pub fn is_furnace(kind: Kind) -> bool {
    matches!(kind, Kind::Furnace | Kind::Smoker | Kind::BlastFurnace)
}

pub fn furnace_layout(kind: Kind) -> Option<&'static FurnaceLayout> {
    match kind {
        Kind::Furnace => Some(&FURNACE),
        Kind::Smoker => Some(&SMOKER),
        Kind::BlastFurnace => Some(&BLAST_FURNACE),
        _ => None,
    }
}

/// The recipe type a furnace cooks.
pub fn cook_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Smoker => "minecraft:smoking",
        Kind::BlastFurnace => "minecraft:blasting",
        _ => "minecraft:smelting",
    }
}

/// The jar's texts for a furnace's zones (cinematic.<kind>.zone.N, .empty.N, .not_cookable, .progress).
pub fn zone_title(kind: Kind, slot: usize) -> &'static str {
    match (kind, slot) {
        (Kind::Smoker, 0) => "Window \u{2014} to cook",
        (Kind::Smoker, 1) => "Behind the grate \u{2014} fuel",
        (Kind::Smoker, _) => "Ready",
        (Kind::BlastFurnace, 0) => "Charging from the top \u{2014} ore",
        (Kind::BlastFurnace, 1) => "Front grate \u{2014} fuel",
        (Kind::BlastFurnace, _) => "Sill \u{2014} finished ingots",
        (_, 0) => "Upper opening \u{2014} to smelt",
        (_, 1) => "Fire box \u{2014} fuel",
        _ => "Done",
    }
}

pub fn zone_empty(kind: Kind, slot: usize) -> &'static str {
    match (kind, slot) {
        (Kind::Smoker, 0) => "Put in raw food",
        (Kind::Smoker, 1) => "Put coal, wood or other fuel behind the grate",
        (Kind::Smoker, _) => "Cooked food comes out here",
        (Kind::BlastFurnace, 0) => "Load ore and metal gear here",
        (Kind::BlastFurnace, 1) => "Put coal or other fuel behind the grate",
        (Kind::BlastFurnace, _) => "Ingots come out on this sill",
        (_, 0) => "Put in what you want smelted",
        (_, 1) => "Put coal, wood or other fuel here",
        _ => "Smelted items come out here",
    }
}

pub fn not_cookable(kind: Kind) -> &'static str {
    match kind {
        Kind::Smoker => "A smoker only cooks food",
        Kind::BlastFurnace => "A blast furnace only melts ores and metal",
        _ => "This won't smelt into anything",
    }
}

pub fn progress_text(kind: Kind, percent: u32) -> String {
    let word = match kind {
        Kind::Smoker => "Cooking",
        Kind::BlastFurnace => "Melting",
        _ => "Smelting",
    };
    format!("{word}: {percent}%")
}

pub const NOT_FUEL: &str = "This won't burn";
pub const NO_ROOM: &str = "Nothing can be put here";
pub const FURNACE_TAKE: &str = "Click to take \u{b7} Shift-click to inventory";

pub fn burning_text(ticks: i32) -> String {
    format!("Burns for {} more s", (ticks.max(0) + 19) / 20)
}

/// The control hint of each station (cinematic.hint.*).
pub fn station_hint(kind: Kind) -> &'static str {
    match kind {
        Kind::Shelf if state().carrying => "LMB on an empty place \u{2014} put the book in  \u{b7}  Esc \u{2014} step away",
        Kind::Shelf => "LMB on a book \u{2014} take it  \u{b7}  Esc \u{2014} step away",
        Kind::Campfire => "Food onto a free spot of the fire  \u{b7}  Esc \u{2014} stand up",
        Kind::Table => HINTS,
        Kind::Furnace => "Fuel goes in below, things to smelt above  \u{b7}  LMB take / put  \u{b7}  RMB one at a time  \u{b7}  Shift to inventory  \u{b7}  Esc step back",
        Kind::Smoker => "Fuel behind the grate, food into the window  \u{b7}  LMB take / put  \u{b7}  RMB one at a time  \u{b7}  Shift to inventory  \u{b7}  Esc step back",
        Kind::BlastFurnace => "Ore goes in from the top, fuel behind the grate  \u{b7}  LMB take / put  \u{b7}  RMB one at a time  \u{b7}  Shift to inventory  \u{b7}  Esc step back",
        Kind::Anvil => "Item on the anvil, what goes into it on top  \u{b7}  LMB on them \u{2014} forge  \u{b7}  RMB \u{2014} take  \u{b7}  the name above \u{2014} rename  \u{b7}  Esc \u{2014} step away",
        Kind::Grindstone => "Lay the item on the wheel  \u{b7}  LMB on it \u{2014} grind (enchantments come off)  \u{b7}  RMB \u{2014} take  \u{b7}  Esc \u{2014} step away",
        Kind::Smithing => "Template left, gear in the middle, material right  \u{b7}  LMB on the gear \u{2014} smith  \u{b7}  RMB \u{2014} take  \u{b7}  Esc \u{2014} step away",
        Kind::Stonecutter => "Lay stone by the saw  \u{b7}  pick a shape round the blade \u{2014} cut one  \u{b7}  Shift \u{2014} cut the stack  \u{b7}  wheel \u{2014} more shapes  \u{b7}  Esc \u{2014} step away",
        Kind::Loom => "Banner on the warp, dye right, pattern left  \u{b7}  pick a pattern on the board \u{2014} weave  \u{b7}  wheel \u{2014} more  \u{b7}  Esc \u{2014} step away",
        Kind::Cartography => "Map in the middle, paper, glass or an empty map on the right  \u{b7}  click the map \u{2014} work it  \u{b7}  Esc \u{2014} step away",
        Kind::Enchanting => "Lay the item on the table  \u{b7}  lapis into the blue rune right of the book  \u{b7}  touch a rune to enchant  \u{b7}  Esc step back",
        Kind::Brewing => "Bottles on the feet, ingredient in the bowl, blaze powder at the foot  \u{b7}  click \u{2014} take  \u{b7}  Esc \u{2014} step away",
        Kind::Lectern => "Click the right page \u{2014} forward, the left \u{2014} back  \u{b7}  the wheel and arrow keys turn pages too  \u{b7}  Shift+click \u{2014} take the book  \u{b7}  Esc \u{2014} step away",
        Kind::Jukebox => "Take a record from the panel at the top and click the jukebox \u{b7} Esc \u{2014} step away",
        Kind::Beacon => "Choose a power over the beacon  \u{b7}  payment on the glass  \u{b7}  click the payment \u{2014} light the beacon  \u{b7}  right-click it \u{2014} take it  \u{b7}  Esc \u{2014} step away",
        Kind::Crafter => "Items into the grid's windows  \u{b7}  empty hand on an empty window \u{2014} shut or open it  \u{b7}  crafts on a redstone signal  \u{b7}  Esc \u{2014} step away",
    }
}

fn block_mid(t: [i32; 3]) -> DVec3 {
    DVec3::new(f64::from(t[0]) + 0.5, f64::from(t[1]), f64::from(t[2]) + 0.5)
}

/// FurnaceZones.point: on the front face, `u` across (0 at the player's left), `v` up from the
/// block's foot, `out` in front of the face.
pub fn front_point(t: [i32; 3], yaw: f32, u: f64, v: f64, out: f64) -> DVec3 {
    block_mid(t) - forward(yaw) * (0.5 + out) + right(yaw) * (u - 0.5) + DVec3::Y * v
}

/// FurnaceZones.top: on the top, `u` across, `v` from the back (0) to the front (1), `h` above it.
pub fn top_point(t: [i32; 3], yaw: f32, u: f64, v: f64, h: f64) -> DVec3 {
    block_mid(t) - forward(yaw) * (v - 0.5) + right(yaw) * (u - 0.5) + DVec3::Y * (1.0 + h)
}

pub fn face_point(face: Face, t: [i32; 3], yaw: f32, u: f64, v: f64, off: f64) -> DVec3 {
    match face {
        Face::Front => front_point(t, yaw, u, v, off),
        Face::Top => top_point(t, yaw, u, v, off),
    }
}

/// The zone the cursor's ray meets first, by its slot.
pub fn furnace_pick(pose: Pose, t: [i32; 3], yaw: f32, kind: Kind, n: (f32, f32), fov: f32, aspect: f32) -> Option<usize> {
    let layout = furnace_layout(kind)?;
    let o = pose.0;
    let d = ray(pose, n, fov, aspect);
    let side = -forward(yaw);
    let mid = block_mid(t);
    let mut best: Option<(f64, usize)> = None;
    let mut consider = |face: Face, dist: f64, u: f64, v: f64| {
        if dist <= 0.0 {
            return;
        }
        for z in &layout.zones {
            if z.face == face && u >= z.u0 && u <= z.u1 && v >= z.v0 && v <= z.v1 && best.is_none_or(|b| dist < b.0) {
                best = Some((dist, z.slot));
            }
        }
    };
    // The front face, half a block towards the player.
    let denom = d.dot(side);
    if denom < -1e-6 {
        let dist = (mid + side * 0.5 - o).dot(side) / denom;
        let hit = o + d * dist;
        consider(Face::Front, dist, (hit - mid).dot(right(yaw)) + 0.5, hit.y - mid.y);
    }
    // The top.
    if d.y < -1e-6 {
        let dist = (mid.y + 1.0 - o.y) / d.y;
        let hit = o + d * dist;
        consider(Face::Top, dist, (hit - mid).dot(right(yaw)) + 0.5, (hit - mid).dot(side) + 0.5);
    }
    best.map(|b| b.1)
}

/// The furnace zone under the cursor, by its slot (0 to smelt, 1 fuel, 2 the result).
pub fn furnace_hover(n: (f32, f32)) -> Option<usize> {
    let s = state();
    if s.phase != Phase::Inside || !is_furnace(s.kind) {
        return None;
    }
    let pose = s.last?;
    furnace_pick(pose, s.table, s.yaw, s.kind, n, s.lens.0, s.lens.1)
}

/// FurnaceStation.rest: the hands wait low on the front of a furnace or a smoker, worked crouching,
/// and on the top of a blast furnace.
pub fn furnace_rest(kind: Kind, side: usize, t: [i32; 3], yaw: f32) -> DVec3 {
    let right_hand = side == 1;
    if kind == Kind::BlastFurnace {
        let u = if right_hand { 1.0 - 0.15 } else { 0.15 };
        top_point(t, yaw, u, 1.0 - 0.13, 0.06) + DVec3::Y * (0.062 - 0.06)
    } else {
        let u = if right_hand { 1.0 - 0.2 } else { 0.2 };
        front_point(t, yaw, u, 0.09, 0.36)
    }
}

/// The middle of a zone's lit rectangle, on its face.
pub fn zone_center(z: &Zone, t: [i32; 3], yaw: f32) -> DVec3 {
    face_point(z.face, t, yaw, (z.hl[0] + z.hl[1]) * 0.5, (z.hl[2] + z.hl[3]) * 0.5, 0.0)
}

/// FurnaceStation.handTarget: the hand stops a little in front of the zone (further with a stack in
/// it), a little above it.
pub fn furnace_reach(kind: Kind, slot: usize, t: [i32; 3], yaw: f32, carrying: bool, from: DVec3) -> Option<DVec3> {
    let z = furnace_layout(kind)?.zones.iter().find(|z| z.slot == slot)?;
    let at = zone_center(z, t, yaw);
    let flat = DVec3::new(at.x - from.x, 0.0, at.z - from.z);
    let dir = if flat.length_squared() > 1e-9 { flat.normalize() } else { forward(yaw) };
    Some(match z.face {
        Face::Top => at + DVec3::Y * (if carrying { 0.075 } else { 0.045 } + 0.062) - dir * 0.07,
        Face::Front => at - forward(yaw) * if carrying { 0.11 } else { 0.075 } - dir * 0.07 + DVec3::Y * 0.035,
    })
}

/// The lit frame round a zone: four thin strips just in front of its face.
pub fn zone_frame(z: &Zone, t: [i32; 3], yaw: f32) -> Vec<[DVec3; 4]> {
    let [u0, u1, v0, v1] = z.hl;
    let w = 0.012;
    let p = |u: f64, v: f64| face_point(z.face, t, yaw, u, v, 0.003);
    let strip = |a: (f64, f64), b: (f64, f64)| [p(a.0, a.1), p(b.0, a.1), p(b.0, b.1), p(a.0, b.1)];
    vec![
        strip((u0, v0), (u1, v0 + w)),
        strip((u0, v1 - w), (u1, v1)),
        strip((u0, v0), (u0 + w, v1)),
        strip((u1 - w, v0), (u1, v1)),
    ]
}

/// ItemPlacer.copies: a block lies alone, flat items in up to four layers by how many there are.
pub fn bed_copies(count: u32, block: bool, max: u32) -> u32 {
    let n = if block {
        1
    } else if count >= 33 {
        4
    } else if count >= 17 {
        3
    } else if count >= 2 {
        2
    } else {
        1
    };
    n.min(max.max(1))
}

/// Where a bed's copies lie.
pub fn bed_spots(b: &Bed, t: [i32; 3], yaw: f32, copies: u32) -> Vec<DVec3> {
    (0..copies)
        .map(|k| {
            let u = b.u0 + (b.u1 - b.u0) * (f64::from(k) + 0.5) / f64::from(copies);
            match b.face {
                Face::Front => front_point(t, yaw, u, b.floor, -(b.depth + 0.14)),
                Face::Top => top_point(t, yaw, u, b.floor, -b.depth),
            }
        })
        .collect()
}

/// Moves the hands at a furnace: the right one reaches for the zone under the cursor.
pub fn step_furnace_hands(slot: Option<usize>, now: Instant, seconds: f64) -> Option<[[DVec3; 2]; 2]> {
    let (kind, t, yaw, carrying, from) = {
        let s = state();
        (s.kind, s.table, s.yaw, s.carrying, s.hands[1])
    };
    let sway = [
        DVec3::new(0.0, 0.003 * (seconds * 1.7).sin(), 0.0),
        DVec3::new(0.0, 0.003 * (seconds * 1.5 + 2.0).sin(), 0.0),
    ];
    let mut targets = [furnace_rest(kind, 0, t, yaw) + sway[0], furnace_rest(kind, 1, t, yaw) + sway[1]];
    let mut reaching = None;
    if let Some(slot) = slot {
        let from = from.unwrap_or(targets[1]);
        if let Some(at) = furnace_reach(kind, slot, t, yaw, carrying, from) {
            targets[1] = at;
            reaching = Some(1);
        }
    }
    step_hands_to(targets, reaching, now)
}

/// The panel's slot or a furnace zone (300 to smelt, 301 fuel, 302 the result) under the cursor.
pub fn furnace_slot_at(w: f32, h: f32, x: f32, y: f32) -> Option<usize> {
    let e = expanded();
    if inside(panel(w, e), x, y) {
        return panel_slot(w, x, y, e);
    }
    furnace_hover(ndc((x, y), w, h)).map(|z| 300 + z)
}

// ------------------------------------------------------------------ the smithing and cartography tables

/// A place on a station's top (SmithingStation.LAYOUT, CartographyStation.LAYOUT): its input, how
/// far to the right and towards the player of the top's middle it is, and its radius.
#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub slot: usize,
    pub x: f64,
    pub z: f64,
    pub r: f64,
    /// Height above the block's top (negative: below it, for a brewing stand's bottles).
    pub y: f64,
}

/// The smithing table: the template on the left, the gear in the middle, the material on the right
/// (sizes 0.22, 0.34, 0.2 at 0.55).
pub const SMITHING_SPOTS: [Spot; 3] = [
    Spot { slot: 0, x: -0.26, z: -0.12, r: 0.22 * 0.55, y: 0.0 },
    Spot { slot: 1, x: 0.0, z: 0.06, r: 0.34 * 0.55, y: 0.0 },
    Spot { slot: 2, x: 0.27, z: -0.12, r: 0.2 * 0.55, y: 0.0 },
];

/// The cartography table: the map in the middle, what works it on the right.
pub const CARTOGRAPHY_SPOTS: [Spot; 2] = [Spot { slot: 0, x: -0.04, z: 0.04, r: 0.28, y: 0.0 }, Spot { slot: 1, x: 0.34, z: -0.1, r: 0.132, y: 0.0 }];

/// The grindstone: the item lies on the wheel, the second one beside it, nearer.
pub const GRINDSTONE_SPOTS: [Spot; 2] = [Spot { slot: 0, x: 0.0, z: -0.05, r: 0.16, y: 0.0 }, Spot { slot: 1, x: 0.0, z: 0.22, r: 0.12, y: -0.18 }];

/// The enchanting table: the item lies before the book, the lapis in the blue rune on its right.
// EnchantFx.itemCenter (0.62 over the top, at 0.75) and lapisCorner (0.44 right, 0.18 out, 0.27 up).
pub const ENCHANTING_SPOTS: [Spot; 2] = [Spot { slot: 0, x: 0.0, z: 0.0, r: 0.16, y: -0.25 + 0.62 }, Spot { slot: 1, x: 0.44, z: 0.18, r: 0.11, y: -0.25 + 0.27 }];

/// The beacon: the payment lies on the glass.
pub const BEACON_SPOTS: [Spot; 1] = [Spot { slot: 0, x: 0.0, z: 0.27, r: 0.2, y: 0.14 }];

/// The brewing stand (BrewingStation.FEET at 4, 12 and 12, 8 pixels): the three bottles round its
/// foot, the ingredient in the bowl on top, the blaze powder at the foot's front.
pub const BREWING_SPOTS: [Spot; 5] = [
    Spot { slot: 0, x: -0.25, z: 0.0, r: 0.11, y: -0.78 },
    Spot { slot: 1, x: 0.22, z: -0.2, r: 0.11, y: -0.78 },
    Spot { slot: 2, x: 0.22, z: 0.2, r: 0.11, y: -0.78 },
    Spot { slot: 3, x: 0.0, z: 0.0, r: 0.12, y: -0.1 },
    Spot { slot: 4, x: 0.0, z: 0.32, r: 0.1, y: -0.95 },
];

/// The loom (SpatialAppearance.Frame for LOOM: LoomStation.CORNERS from the top's middle): the
/// banner over the warp, the dye 0.37 right and the pattern 0.37 left, both 0.34 out.
pub const LOOM_SPOTS: [Spot; 3] = [
    Spot { slot: 0, x: 0.0, z: 0.0, r: 0.17, y: 0.0 },
    Spot { slot: 1, x: 0.37, z: 0.34, r: 0.11, y: 0.0 },
    Spot { slot: 2, x: -0.37, z: 0.34, r: 0.11, y: 0.0 },
];

/// The loom's pattern board: where the game's own loom patterns are drawn (its screen's origin;
/// the buttons are at 60, 13, four by four of 14, the scroller at 119, 9).
pub fn loom_board(w: f32, h: f32) -> (f32, f32) {
    ((w - 182.0).floor(), (h * 0.5 - 52.0).floor())
}

/// The cursor is on the board's buttons or scroller.
pub fn on_loom_board(w: f32, h: f32, x: f32, y: f32) -> bool {
    let (ox, oy) = loom_board(w, h);
    inside([ox + 56.0, oy + 4.0, 80.0, 70.0], x, y)
}

/// The chiseled bookshelf's six places on its front (ChiseledBookShelfBlock.getHitSlot: three
/// columns, two rows, the top row first), just out of the face.
pub const SHELF_SPOTS: [Spot; 6] = [
    Spot { slot: 0, x: -1.0 / 3.0, z: 0.51, r: 0.15, y: -0.25 },
    Spot { slot: 1, x: 0.0, z: 0.51, r: 0.15, y: -0.25 },
    Spot { slot: 2, x: 1.0 / 3.0, z: 0.51, r: 0.15, y: -0.25 },
    Spot { slot: 3, x: -1.0 / 3.0, z: 0.51, r: 0.15, y: -0.75 },
    Spot { slot: 4, x: 0.0, z: 0.51, r: 0.15, y: -0.75 },
    Spot { slot: 5, x: 1.0 / 3.0, z: 0.51, r: 0.15, y: -0.75 },
];

/// Items a chiseled bookshelf takes (the `bookshelf_books` tag).
pub fn is_book(item: &str) -> bool {
    matches!(item, "minecraft:book" | "minecraft:written_book" | "minecraft:writable_book" | "minecraft:enchanted_book" | "minecraft:knowledge_book")
}

/// The world point of a spot (for a use on it) and the face it is on (towards the player).
pub fn spot_use(kind: Kind, slot: usize) -> Option<([f64; 3], [i32; 3])> {
    let s = state();
    let spot = surface_spots_of(&s, kind).iter().find(|p| p.slot == slot).copied()?;
    let p = spot_point(s.table, s.yaw, &spot);
    let n = -forward(s.yaw);
    let face = if kind == Kind::Shelf { [n.x.round() as i32, 0, n.z.round() as i32] } else { [0, 1, 0] };
    Some(([p.x, p.y, p.z], face))
}

fn surface_spots_of(s: &State, kind: Kind) -> &'static [Spot] {
    if kind == Kind::Campfire { s.campfire_spots } else { spots_of(kind) }
}

pub fn is_surface(kind: Kind) -> bool {
    matches!(kind, Kind::Smithing | Kind::Cartography | Kind::Grindstone | Kind::Brewing | Kind::Enchanting | Kind::Beacon | Kind::Campfire | Kind::Loom | Kind::Shelf)
}

/// Where a station's spots are in the menu: a brewing stand is a container (300 on), the others
/// stations (600 on).
pub fn spot_base(kind: Kind) -> usize {
    if kind == Kind::Brewing { 300 } else { 600 }
}

/// Whether the worked spot gives the result (650).
pub fn has_work(kind: Kind) -> bool {
    !matches!(kind, Kind::Brewing | Kind::Enchanting | Kind::Beacon | Kind::Campfire | Kind::Shelf)
}

pub fn surface_spots(kind: Kind) -> &'static [Spot] {
    if kind == Kind::Campfire { state().campfire_spots } else { spots_of(kind) }
}

/// The fixed spots of a station (the campfire's follow its facing).
fn spots_of(kind: Kind) -> &'static [Spot] {
    match kind {
        Kind::Smithing => &SMITHING_SPOTS,
        Kind::Cartography => &CARTOGRAPHY_SPOTS,
        Kind::Grindstone => &GRINDSTONE_SPOTS,
        Kind::Brewing => &BREWING_SPOTS,
        Kind::Enchanting => &ENCHANTING_SPOTS,
        Kind::Beacon => &BEACON_SPOTS,
        Kind::Loom => &LOOM_SPOTS,
        Kind::Shelf => &SHELF_SPOTS,
        _ => &[],
    }
}

/// The spot that is worked: the smithing table's gear, the cartography table's map.
pub fn work_spot(kind: Kind) -> usize {
    if kind == Kind::Smithing { 1 } else { 0 }
}

pub fn spot_point(t: [i32; 3], yaw: f32, s: &Spot) -> DVec3 {
    top_center(t) + right(yaw) * s.x - forward(yaw) * s.z + DVec3::Y * s.y
}

/// The spot the cursor's ray meets on the top.
pub fn surface_pick(pose: Pose, t: [i32; 3], yaw: f32, kind: Kind, n: (f32, f32), fov: f32, aspect: f32) -> Option<usize> {
    let d = ray(pose, n, fov, aspect);
    let mut best: Option<(f64, usize)> = None;
    for s in surface_spots(kind) {
        let c = spot_point(t, yaw, s);
        let along = (c - pose.0).dot(d);
        if along <= 0.0 {
            continue;
        }
        let miss = (pose.0 + d * along).distance(c);
        if miss <= s.r && best.is_none_or(|b| along < b.0) {
            best = Some((along, s.slot));
        }
    }
    best.map(|b| b.1)
}

pub fn surface_hover(n: (f32, f32)) -> Option<usize> {
    let (pose, table, yaw, kind, lens) = {
        let s = state();
        if s.phase != Phase::Inside || !is_surface(s.kind) {
            return None;
        }
        (s.last?, s.table, s.yaw, s.kind, s.lens)
    };
    surface_pick(pose, table, yaw, kind, n, lens.0, lens.1)
}

/// SmithingStation.rest, CartographyStation.rest: on the near edge of the top, 0.25 (0.26) aside.
pub fn surface_rest(kind: Kind, side: usize, t: [i32; 3], yaw: f32) -> DVec3 {
    if kind == Kind::Campfire {
        // CampfireStation.rest: the right hand on the right knee, the left on the left one.
        let seat = DVec3::new(f64::from(t[0]) + 0.5, f64::from(t[1]), f64::from(t[2]) + 0.5) - forward(yaw) * 1.6;
        return if side == 1 {
            seat + forward(yaw) * (0.36 + 0.02) + right(yaw) * 0.17 + DVec3::Y * (0.46 + 0.075)
        } else {
            seat + forward(yaw) * (0.45 - 0.08) - right(yaw) * 0.13 + DVec3::Y * (0.13 + 0.085)
        };
    }
    let f = if side == 0 { -1.0 } else { 1.0 };
    let aside = if kind == Kind::Cartography { 0.26 } else { 0.25 };
    let low = match kind {
        Kind::Brewing => -0.85,
        Kind::Grindstone => -0.2,
        Kind::Enchanting => -0.25,
        Kind::Shelf => -0.62,
        _ => 0.0,
    };
    top_center(t) - forward(yaw) * 0.4 + right(yaw) * (aside * f) + DVec3::Y * (0.062 + low)
}

pub fn step_surface_hands(spot: Option<usize>, now: Instant, seconds: f64) -> Option<[[DVec3; 2]; 2]> {
    let (kind, t, yaw, carrying) = {
        let s = state();
        (s.kind, s.table, s.yaw, s.carrying)
    };
    let sway = [
        DVec3::new(0.0, 0.003 * (seconds * 1.7).sin(), 0.0),
        DVec3::new(0.0, 0.003 * (seconds * 1.5 + 2.0).sin(), 0.0),
    ];
    let mut targets = [surface_rest(kind, 0, t, yaw) + sway[0], surface_rest(kind, 1, t, yaw) + sway[1]];
    let mut reaching = None;
    if let Some(s) = spot.and_then(|slot| surface_spots(kind).iter().find(|s| s.slot == slot)) {
        let side = usize::from(s.x >= -0.1);
        let lift = if carrying { 0.075 } else { 0.045 };
        targets[side] = spot_point(t, yaw, s) + DVec3::Y * (0.062 + lift * 0.5) - forward(yaw) * 0.075;
        reaching = Some(side);
    }
    step_hands_to(targets, reaching, now)
}

/// The panel's slot or, on the top, a spot's input (600 on); the worked spot gives the result (650)
/// to an empty hand once there is one.
pub fn surface_slot_at(w: f32, h: f32, x: f32, y: f32) -> Option<usize> {
    let e = expanded();
    if inside(panel(w, e), x, y) {
        return panel_slot(w, x, y, e);
    }
    let spot = surface_hover(ndc((x, y), w, h))?;
    let (kind, result, carrying) = {
        let s = state();
        (s.kind, s.result_shown, s.carrying)
    };
    Some(if has_work(kind) && spot == work_spot(kind) && result && !carrying { 650 } else { spot_base(kind) + spot })
}

/// The lit frame round a spot: four strips just over the top.
pub fn spot_frame(s: &Spot, t: [i32; 3], yaw: f32) -> Vec<[DVec3; 4]> {
    let c = spot_point(t, yaw, s) + DVec3::Y * 0.003;
    let (r, f) = (right(yaw), forward(yaw));
    let a = s.r * 0.9;
    let w = 0.012;
    let q = |x0: f64, z0: f64, x1: f64, z1: f64| [c + r * x0 + f * z0, c + r * x1 + f * z0, c + r * x1 + f * z1, c + r * x0 + f * z1];
    vec![q(-a, -a, a, -a + w), q(-a, a - w, a, a), q(-a, -a, -a + w, a), q(a - w, -a, a, a)]
}

/// The jar's texts for the smithing and cartography tables.
pub fn spot_title(kind: Kind, slot: usize) -> &'static str {
    match (kind, slot) {
        (Kind::Enchanting, 0) => "Lay here what you want to enchant",
        (Kind::Enchanting, _) => "Put lapis lazuli into this blue rune",
        (Kind::Beacon, _) => "Lay the payment on the beacon",
        (Kind::Loom, 0) => "A banner \u{2014} over the warp",
        (Kind::Loom, 1) => "A dye",
        (Kind::Loom, _) => "A banner pattern",
        (Kind::Brewing, 0..=2) => "A water bottle or a potion",
        (Kind::Brewing, 3) => "The ingredient \u{2014} into the bowl on top",
        (Kind::Brewing, _) => "Blaze powder \u{2014} the fuel",
        (Kind::Grindstone, 0) => "Lay the item on the wheel",
        (Kind::Grindstone, _) => "A second item to grind into one",
        (Kind::Smithing, 0) => "Smithing template",
        (Kind::Smithing, 1) => "Gear: armour, a weapon or a tool",
        (Kind::Smithing, _) => "Material: an ingot or a mineral",
        (_, 0) => "A filled map goes here",
        _ => "Paper, glass or an empty map goes here",
    }
}

pub fn surface_refuse(kind: Kind) -> &'static str {
    match kind {
        Kind::Smithing => "The smithing table has no use for this",
        Kind::Grindstone => "The grindstone only takes enchanted or worn items",
        Kind::Brewing => "That doesn't go here",
        Kind::Enchanting => "Only lapis lazuli fits here",
        Kind::Beacon => "A beacon takes only iron, gold, emerald, diamond or netherite",
        Kind::Loom => "The loom has no use for this",
        _ => "The cartography table has no use for this",
    }
}

pub fn surface_work(kind: Kind) -> &'static str {
    if kind == Kind::Grindstone {
        "LMB \u{2014} grind  \u{b7}  RMB \u{2014} take  \u{b7}  Shift \u{2014} straight into the inventory"
    } else if kind == Kind::Loom {
        "LMB \u{2014} weave  \u{b7}  Shift \u{2014} straight into the inventory"
    } else if kind == Kind::Smithing {
        "LMB \u{2014} smith  \u{b7}  RMB \u{2014} take  \u{b7}  Shift \u{2014} straight into the inventory"
    } else {
        "LMB \u{2014} work it  \u{b7}  Shift \u{2014} straight into the inventory"
    }
}

pub const SURFACE_TAKE: &str = "Click \u{2014} take";

pub fn smithing_missing(slot: usize) -> &'static str {
    match slot {
        0 => "A template is missing",
        1 => "The gear is missing",
        _ => "A material is missing",
    }
}

/// What the cartography table's addition does to the map.
pub fn cartography_effect(addition: &str) -> &'static str {
    match addition {
        "minecraft:paper" => "Paper: the map will cover more land",
        "minecraft:glass_pane" => "Glass: the map is locked and won't change",
        "minecraft:map" => "Empty map: makes a copy",
        _ => "Lay paper, glass or an empty map on the right",
    }
}


// ------------------------------------------------------------------ runes and powers

/// Where a world point is on the screen (GUI units), as the scene shows it now.
pub fn screen_of(p: DVec3, w: f32, h: f32) -> Option<(f32, f32)> {
    let s = state();
    let pose = s.last?;
    let n = project(pose, p, s.lens.0, s.lens.1)?;
    Some(((n.0 + 1.0) * 0.5 * w, (1.0 - n.1) * 0.5 * h))
}

/// The enchanting table's three offers float over it: the first high in the middle, the others
/// to the left and the right.
pub fn rune_points(t: [i32; 3], yaw: f32) -> [DVec3; 3] {
    // SpatialAppearance.Frame.base for ENCHANTING's sigils: from the top (0.75), the middle one 0.92
    // up and 0.12 back, the others 0.74 up, 0.02 out and 0.4 to the side.
    let c = top_center(t) - DVec3::Y * 0.25;
    [
        c + DVec3::Y * 0.74 - forward(yaw) * 0.02 - right(yaw) * 0.4,
        c + DVec3::Y * 0.92 + forward(yaw) * 0.12,
        c + DVec3::Y * 0.74 - forward(yaw) * 0.02 + right(yaw) * 0.4,
    ]
}

/// A rune's (or power's) radius on the screen, in GUI units.
pub const RUNE_RADIUS: f32 = 12.0;

/// The offer whose rune is under the cursor.
pub fn rune_at(x: f32, y: f32, w: f32, h: f32) -> Option<usize> {
    let (kind, t, yaw, inside) = {
        let s = state();
        (s.kind, s.table, s.yaw, s.phase == Phase::Inside)
    };
    if kind != Kind::Enchanting || !inside {
        return None;
    }
    rune_points(t, yaw).iter().position(|p| screen_of(*p, w, h).is_some_and(|(sx, sy)| (sx - x).hypot(sy - y) <= RUNE_RADIUS))
}

/// The beacon's powers (BeaconScreen's buttons, in the game's own beacon screen units with its
/// shift): the effect, the pyramid level it needs, primary or not, the button's middle, and
/// where it floats (right, up of the beacon's top).
pub struct Power {
    pub effect: &'static str,
    pub level: i32,
    pub primary: bool,
    pub upgrade: bool,
    pub button: (f32, f32),
    pub at: (f64, f64),
}

const B: f32 = -27.0 + 11.0;
pub const POWERS: [Power; 7] = [
    Power { effect: "speed", level: 1, primary: true, upgrade: false, button: (53.0 + B, 22.0 + 11.0), at: (-0.8, 0.2) },
    Power { effect: "haste", level: 1, primary: true, upgrade: false, button: (77.0 + B, 22.0 + 11.0), at: (-0.45, 0.2) },
    Power { effect: "resistance", level: 2, primary: true, upgrade: false, button: (53.0 + B, 47.0 + 11.0), at: (-0.8, 0.51) },
    Power { effect: "jump_boost", level: 2, primary: true, upgrade: false, button: (77.0 + B, 47.0 + 11.0), at: (-0.45, 0.51) },
    Power { effect: "strength", level: 3, primary: true, upgrade: false, button: (65.0 + B, 72.0 + 11.0), at: (-0.62, 0.82) },
    Power { effect: "regeneration", level: 4, primary: false, upgrade: false, button: (144.0 + B, 47.0 + 11.0), at: (0.6, 0.36) },
    Power { effect: "regeneration", level: 4, primary: false, upgrade: true, button: (168.0 + B, 47.0 + 11.0), at: (0.6, 0.67) },
];

/// The beacon screen's confirm button, where a click on the payment lights the beacon.
pub const BEACON_CONFIRM: (f32, f32) = (164.0 + B, 107.0 + 11.0);

/// RemainingLayouts.base for BEACON: from the top, `at.0` to the side, `at.1` up and
/// 0.14 - 0.08 |at.0| out.
pub fn power_point(t: [i32; 3], yaw: f32, p: &Power) -> DVec3 {
    top_center(t) - forward(yaw) * (0.14 - 0.08 * p.at.0.abs()) + right(yaw) * p.at.0 + DVec3::Y * p.at.1
}

pub fn power_at(x: f32, y: f32, w: f32, h: f32) -> Option<usize> {
    let (kind, t, yaw, inside) = {
        let s = state();
        (s.kind, s.table, s.yaw, s.phase == Phase::Inside)
    };
    if kind != Kind::Beacon || !inside {
        return None;
    }
    POWERS.iter().position(|p| screen_of(power_point(t, yaw, p), w, h).is_some_and(|(sx, sy)| (sx - x).hypot(sy - y) <= RUNE_RADIUS))
}

/// A filled disc in rows, for a rune's or a power's background.
pub fn disc(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, r: f32, color: u32) {
    let rows = (r * 2.0).ceil() as i32;
    for i in 0..rows {
        let dy = i as f32 + 0.5 - r;
        let half = (r * r - dy * dy).max(0.0).sqrt();
        batch.fill(atlas, cx - half, cy - r + i as f32, half * 2.0, 1.0, color);
    }
}

pub fn roman(n: u32) -> &'static str {
    match n {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        _ => "",
    }
}


// ------------------------------------------------------------------ the campfire

/// A campfire or a soul campfire.
pub fn is_campfire(block: &str) -> bool {
    matches!(block, "minecraft:campfire" | "minecraft:soul_campfire")
}

/// Direction.get2DDataValue of a `facing` value (south 0, west 1, north 2, east 3).
pub fn facing_2d(facing: &str) -> i32 {
    match facing {
        "west" => 1,
        "north" => 2,
        "east" => 3,
        _ => 0,
    }
}

/// CampfireStation.vanillaSpot: where the campfire renderer lays food `i`, from the block's
/// bottom middle (0.3125 out on both sides, 0.449 up, turned with the spot's direction).
pub fn campfire_offset(facing: i32, i: usize) -> DVec3 {
    let dir = (i as i32 + facing).rem_euclid(4);
    let y_rot = f64::from(dir) * 90.0;
    let a = (-y_rot).to_radians();
    let (x, z) = (-0.3125f64, -0.3125f64);
    DVec3::new(x * a.cos() + z * a.sin(), 0.44921875, -x * a.sin() + z * a.cos())
}

/// Sit by the campfire at `t` (H): the yaw looks from the player at the fire, unsnapped.
pub fn sit(t: [i32; 3], facing: i32, player: DVec3, now: Instant) {
    let fire = DVec3::new(f64::from(t[0]) + 0.5, f64::from(t[1]), f64::from(t[2]) + 0.5);
    let (dx, dz) = (fire.x - player.x, fire.z - player.z);
    let yaw = if dx * dx + dz * dz < 1e-4 { 0.0 } else { ((-dx).atan2(dz).to_degrees() as f32).rem_euclid(360.0) };
    let spots: Vec<Spot> = (0..4)
        .map(|i| {
            let o = campfire_offset(facing, i);
            Spot { slot: i, x: o.dot(right(yaw)), z: -o.dot(forward(yaw)), r: 0.13, y: o.y - 1.0 }
        })
        .collect();
    let mut s = state();
    s.clicked = Some((t, now, false));
    s.clicked_kind = Kind::Campfire;
    s.clicked_yaw = Some(yaw);
    s.sitting = true;
    s.campfire_spots = Box::leak(spots.into_boxed_slice());
    s.place = None;
}

/// A station with no menu of its own (the chiseled bookshelf): the scene keeps the inventory
/// screen it opens until it closes.
pub fn hold_scene() {
    let mut s = state();
    s.sitting = true;
    s.place = None;
}

/// Sitting by the fire (until the screen closes).
pub fn sitting() -> bool {
    state().sitting
}

/// The inventory closed: up again.
pub fn stand_up() {
    state().sitting = false;
}

/// Put the carried food on the fire (done on the next tick, as the game's own use).
pub fn request_place(t: [i32; 3]) {
    state().place = Some((t, None, true));
}

/// Use the hand on a spot of a station with no menu (a bookshelf's place): `point` and `face` are
/// where the use hits, `carried` whether the carried stack is to be used.
pub fn request_use(t: [i32; 3], point: [f64; 3], face: [i32; 3], carried: bool) {
    state().place = Some((t, Some((point, face)), carried));
}

#[allow(clippy::type_complexity)]
pub fn take_place() -> Option<([i32; 3], Option<([f64; 3], [i32; 3])>, bool)> {
    state().place.take()
}

/// The "[H] Sit by the fire" plate under the crosshair (CampfireHint).
pub fn draw_sit_plate(batch: &mut HudBatch, atlas: &HudAtlas, w: f32, h: f32) {
    let text = "Sit by the fire";
    let tw = HudBatch::text_width(atlas, text);
    let key = 9.0;
    let width = key + 4.0 + tw + 6.0;
    let x = (w * 0.5 - width * 0.5).floor();
    let y = (h * 0.5 + 10.0).floor();
    batch.fill(atlas, x, y, width, 13.0, rgba(16, 16, 16, 150));
    batch.fill(atlas, x + 2.0, y + 2.0, key, key, rgba(220, 220, 220, 255));
    batch.fill(atlas, x + 3.0, y + 3.0, key - 2.0, key - 2.0, rgba(32, 32, 32, 255));
    batch.text(atlas, "H", x + 4.0, y + 3.0, 0xFFFF_FFFF, false);
    batch.text(atlas, text, x + key + 5.0, y + 3.0, 0xFFFF_FFFF, true);
}

// ------------------------------------------------------------------ the table keeps its items

/// What lies on a crafting table the player left (the jar's tables keep their grid): its
/// dimension, place, the yaw it was worked from and the nine cells (item, count).
#[derive(Clone, Debug, PartialEq)]
pub struct KeptTable {
    pub dim: String,
    pub pos: [i32; 3],
    pub yaw: f32,
    pub items: [Option<(String, u32)>; 9],
}

/// The world the kept tables belong to (its folder), whether they were read, and the tables.
static TABLES: Mutex<(String, bool, Vec<KeptTable>)> = Mutex::new((String::new(), false, Vec::new()));

fn tables() -> std::sync::MutexGuard<'static, (String, bool, Vec<KeptTable>)> {
    TABLES.lock().unwrap_or_else(|e| e.into_inner())
}

fn tables_file(root: &str) -> std::path::PathBuf {
    std::path::Path::new(root).join("cinematic_interactions").join("tables.txt")
}

/// One table a line: `dim x y z yaw` and the nine cells, `item count` or `-`, split by `|`.
pub fn tables_to_text(list: &[KeptTable]) -> String {
    let mut out = String::new();
    for t in list {
        out.push_str(&format!("{} {} {} {} {}", t.dim, t.pos[0], t.pos[1], t.pos[2], t.yaw));
        for c in &t.items {
            match c {
                Some((item, n)) => out.push_str(&format!("|{item} {n}")),
                None => out.push_str("|-"),
            }
        }
        out.push('\n');
    }
    out
}

pub fn tables_from_text(text: &str) -> Vec<KeptTable> {
    let mut list = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('|');
        let head: Vec<&str> = parts.next().unwrap_or("").split_whitespace().collect();
        if head.len() != 5 {
            continue;
        }
        let (Ok(x), Ok(y), Ok(z), Ok(yaw)) = (head[1].parse(), head[2].parse(), head[3].parse(), head[4].parse()) else {
            continue;
        };
        let mut items: [Option<(String, u32)>; 9] = Default::default();
        for (i, c) in parts.take(9).enumerate() {
            let mut w = c.split_whitespace();
            if let (Some(item), Some(n)) = (w.next(), w.next().and_then(|n| n.parse().ok())) {
                items[i] = Some((item.to_owned(), n));
            }
        }
        if items.iter().any(Option::is_some) {
            list.push(KeptTable { dim: head[0].to_owned(), pos: [x, y, z], yaw, items });
        }
    }
    list
}

/// The server's world: its kept tables are read once (another world's are dropped first).
pub fn tables_load(root: &str) {
    if root.is_empty() {
        return;
    }
    let mut t = tables();
    if t.1 && t.0 == root {
        return;
    }
    t.0 = root.to_owned();
    t.1 = true;
    t.2 = std::fs::read_to_string(tables_file(root)).map(|s| tables_from_text(&s)).unwrap_or_default();
}

fn tables_save(t: &(String, bool, Vec<KeptTable>)) {
    if t.0.is_empty() {
        return;
    }
    let path = tables_file(&t.0);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, tables_to_text(&t.2));
}

/// A table left with items on it: they stay there. False when this world can't keep them (no
/// folder), so they go back to the player as usual.
pub fn table_keep(root: &str, dim: &str, pos: [i32; 3], yaw: f32, items: [Option<(String, u32)>; 9]) -> bool {
    if root.is_empty() {
        return false;
    }
    tables_load(root);
    let mut t = tables();
    t.2.retain(|k| !(k.dim == dim && k.pos == pos));
    if items.iter().any(Option::is_some) {
        t.2.push(KeptTable { dim: dim.to_owned(), pos, yaw, items });
    }
    tables_save(&t);
    true
}

/// The items kept on a table, taken off it (the table opens with them in its grid).
pub fn table_take(root: &str, dim: &str, pos: [i32; 3]) -> Option<[Option<(String, u32)>; 9]> {
    if root.is_empty() {
        return None;
    }
    tables_load(root);
    let mut t = tables();
    let i = t.2.iter().position(|k| k.dim == dim && k.pos == pos)?;
    let k = t.2.remove(i);
    tables_save(&t);
    Some(k.items)
}

/// The kept tables of the world being played (for drawing their items).
pub fn kept_tables() -> Vec<KeptTable> {
    tables().2.clone()
}

/// The yaw a table is worked from now, if it is the scene's.
pub fn table_yaw(pos: [i32; 3]) -> Option<f32> {
    let s = state();
    (s.kind == Kind::Table && s.table == pos && s.phase != Phase::Idle).then_some(s.yaw)
}

/// The scene is at this table (its own hands draw the grid there).
pub fn scene_at(pos: [i32; 3]) -> bool {
    let s = state();
    s.active && s.kind == Kind::Table && s.table == pos && s.phase != Phase::Idle
}
