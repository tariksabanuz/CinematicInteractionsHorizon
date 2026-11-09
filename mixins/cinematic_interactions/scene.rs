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
pub const VERSION: &str = "0.12.2";
pub const DESCRIPTION: &str = "By tariksabanuz. Right-click a crafting table: you walk up, the camera leans over it, your arms reach \
for the grid and the items lie on the table, and stay there when you leave. Furnaces, smithing and cartography tables, \
grindstones, brewing stands, looms, crafters, stonecutters, enchanting tables, beacons and chiseled bookshelves are worked in \
the world too; press H at a campfire to sit by the fire. Sneak + right-click opens the normal screen.";

/// A camera pose: where the eye is, its yaw and its pitch (degrees, Minecraft's convention).
pub type Pose = (DVec3, f32, f32);

// ------------------------------------------------------------------ settings

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// The black bars while the camera moves (Letterbox bars).
    pub bars: bool,
    /// The control hints at the bottom of the screen.
    pub hints: bool,
    /// The camera's speed in the Mods page: 0 slow, 1 normal, 2 fast.
    pub speed: u8,
    /// A station out of reach says why (else it is silent).
    pub reach_why: bool,
    /// The "[H] Sit by the fire" plate.
    pub invite: bool,
    /// The orbit view key: hold (else toggle).
    pub orbit_hold: bool,
    /// The arms at work: stretched (else relaxed).
    pub arms_stretched: bool,
    /// Stations turned off (bits of [`STATIONS`]): they open their normal screen.
    pub off: u32,
    /// The camera preset: 0 balanced, 1 cinematic, 2 snappy, 3 custom.
    pub preset: u8,
    pub reduce_motion: bool,
    /// The way in: a quick return within the window (else always the full glide).
    pub quick_return: bool,
    /// Percent of the normal approach and leaving speed.
    pub approach: u16,
    pub leaving: u16,
    /// Seconds after leaving in which coming back is quick.
    pub window: u16,
    /// Percent of the camera's lean and of the scene's zoom.
    pub lean: u16,
    pub zoom: u16,
    pub breathing: bool,
    pub shake: bool,
    pub flashes: bool,
    pub developer: bool,
}

pub const DEFAULT: Config = Config {
    bars: true,
    hints: true,
    speed: 1,
    reach_why: true,
    invite: true,
    orbit_hold: true,
    arms_stretched: true,
    off: 0,
    preset: 0,
    reduce_motion: false,
    quick_return: true,
    approach: 100,
    leaving: 100,
    window: 12,
    lean: 100,
    zoom: 100,
    breathing: true,
    shake: true,
    flashes: true,
    developer: false,
};
const SPEEDS: [&str; 3] = ["Slow", "Normal", "Fast"];

/// The stations of the settings' Stations page: kind, name, icon item and group.
pub const STATIONS: [(Kind, &str, &str, u8); 18] = [
    (Kind::Table, "Crafting Table", "minecraft:crafting_table", 0),
    (Kind::Crafter, "Crafter", "minecraft:crafter", 0),
    (Kind::Stonecutter, "Stonecutter", "minecraft:stonecutter", 0),
    (Kind::Loom, "Loom", "minecraft:loom", 0),
    (Kind::Cartography, "Cartography Table", "minecraft:cartography_table", 0),
    (Kind::Furnace, "Furnace", "minecraft:furnace", 1),
    (Kind::Smoker, "Smoker", "minecraft:smoker", 1),
    (Kind::BlastFurnace, "Blast Furnace", "minecraft:blast_furnace", 1),
    (Kind::Campfire, "Campfire", "minecraft:campfire", 1),
    (Kind::Anvil, "Anvil", "minecraft:anvil", 2),
    (Kind::Grindstone, "Grindstone", "minecraft:grindstone", 2),
    (Kind::Smithing, "Smithing Table", "minecraft:smithing_table", 2),
    (Kind::Enchanting, "Enchanting Table", "minecraft:enchanting_table", 3),
    (Kind::Brewing, "Brewing Stand", "minecraft:brewing_stand", 3),
    (Kind::Beacon, "Beacon", "minecraft:beacon", 3),
    (Kind::Lectern, "Lectern", "minecraft:lectern", 4),
    (Kind::Jukebox, "Jukebox", "minecraft:jukebox", 4),
    (Kind::Shelf, "Chiseled bookshelf", "minecraft:chiseled_bookshelf", 4),
];
pub const GROUPS: [&str; 5] = ["Crafting", "Smelting and cooking", "Smithing", "Magic", "Other"];

/// Whether the settings leave this station to the scene.
pub fn station_on(kind: Kind) -> bool {
    let off = state().config.off;
    STATIONS.iter().position(|s| s.0 == kind).is_none_or(|i| off & (1 << i) == 0)
}

pub fn parse(text: &str) -> Config {
    let mut c = DEFAULT;
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        let on = v == "true";
        let n = v.parse::<u32>().unwrap_or(0);
        match k {
            "bars" => c.bars = on,
            "hints" => c.hints = on,
            "speed" => c.speed = (n as u8).min(2),
            "reach_why" => c.reach_why = on,
            "invite" => c.invite = on,
            "orbit_hold" => c.orbit_hold = on,
            "arms_stretched" => c.arms_stretched = on,
            "off" => c.off = n,
            "preset" => c.preset = (n as u8).min(3),
            "reduce_motion" => c.reduce_motion = on,
            "quick_return" => c.quick_return = on,
            "approach" => c.approach = (n as u16).clamp(25, 300),
            "leaving" => c.leaving = (n as u16).clamp(25, 300),
            "window" => c.window = (n as u16).min(60),
            "lean" => c.lean = (n as u16).min(200),
            "zoom" => c.zoom = (n as u16).clamp(50, 150),
            "breathing" => c.breathing = on,
            "shake" => c.shake = on,
            "flashes" => c.flashes = on,
            "developer" => c.developer = on,
            _ => {}
        }
    }
    c
}

pub fn to_text(c: &Config) -> String {
    format!(
        "bars={}\nhints={}\nspeed={}\nreach_why={}\ninvite={}\norbit_hold={}\narms_stretched={}\noff={}\npreset={}\nreduce_motion={}\nquick_return={}\napproach={}\nleaving={}\nwindow={}\nlean={}\nzoom={}\nbreathing={}\nshake={}\nflashes={}\ndeveloper={}\n",
        c.bars, c.hints, c.speed, c.reach_why, c.invite, c.orbit_hold, c.arms_stretched, c.off, c.preset, c.reduce_motion, c.quick_return, c.approach, c.leaving, c.window, c.lean, c.zoom, c.breathing, c.shake, c.flashes, c.developer
    )
}

/// The settings screen's Done: the new settings, saved.
pub fn apply_config(c: Config) {
    let mut s = state();
    s.config = c;
    save(&s);
}

/// Seconds the camera takes to lean over the table (after the walk), and to come back.
pub fn durations(c: &Config) -> (f32, f32) {
    let speed = match c.speed {
        0 => 0.6,
        2 => 1.6,
        _ => 1.0,
    };
    let fast = if c.reduce_motion { 1.5 } else { 1.0 };
    let pace = |p: u16| speed * fast * (f32::from(p) / 100.0).max(0.25);
    (LEAN / pace(c.approach), OUTRO / pace(c.leaving))
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
    /// The scene's own use of a block is going on (not a click that opens a scene).
    own_use: bool,
    move_to: Option<[i32; 3]>,
    /// A book sliding out of (false) or into (true) a shelf place: place, start, item.
    slide: Option<(usize, Instant, bool, String)>,
    /// Moving over to the next station: the camera slides straight to the side, the same yaw.
    sliding: bool,
    /// The station beside the one in the scene that the cursor is over (the jar's "move over").
    neighbour: Option<[i32; 3]>,
    /// The last ritual: the enchanting's light, the beacon's, the anvil's strike.
    burst: Option<(Kind, Instant)>,
    /// The anvil's forge: when it began, Shift, and whether the strike has taken the result.
    forge: Option<(Instant, bool, bool)>,
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
    /// The anvil's rename box is open (the floating name was clicked).
    renaming: bool,
    /// Where the anvil's floating name was drawn last (GUI units), for its click.
    name_rect: Option<[f32; 4]>,
    /// The stonecutter's first shape shown (the wheel pages through them).
    stone_page: usize,
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
    own_use: false,
    move_to: None,
    slide: None,
    sliding: false,
    neighbour: None,
    burst: None,
    forge: None,
    renaming: false,
    name_rect: None,
    stone_page: 0,
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
    let kind = kind_of_block(block, low)?;
    station_on(kind).then_some(kind)
}

fn kind_of_block(block: &str, low: bool) -> Option<Kind> {
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
        // A click on another station while in the scene (moving over): the scene goes there.
        let moved = s.clicked.filter(|c| c.0 != s.table && !c.2 && now.duration_since(c.1).as_secs_f32() < 3.0);
        if let Some(c) = moved {
            s.clicked = None;
            s.kind = s.clicked_kind;
            s.sliding = true;
            Some(c.0)
        } else {
            (!s.plain).then_some(s.table)
        }
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
            s.sliding = false;
            s.hands = [None, None];
            s.selected = None;
            s.walk = None;
            s.renaming = false;
            s.name_rect = None;
            s.stone_page = 0;
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
            // The jar's move over: straight to the left or the right, looking the same way.
            let keep = s.yaw;
            let picked = s.clicked_yaw.take();
            s.yaw = if s.sliding { keep } else { picked.unwrap_or_else(|| facing(player, t, view_yaw)) };
            s.phase = Phase::Enter;
            s.since = Some(now);
            s.renaming = false;
            s.name_rect = None;
            s.stone_page = 0;
        }
        (Phase::Enter | Phase::Inside, None) => {
            s.from = s.last;
            s.phase = Phase::Exit;
            s.since = Some(now);
            s.renaming = false;
            s.name_rect = None;
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
                    s.sliding = false;
                }
                let ws = if walk.is_some() { WALK_SHARE } else { 0.0 };
                let lift = if s.sliding || s.config.reduce_motion { 0.0 } else { 0.045 * f64::from(s.config.lean) / 100.0 };
                Some(start.map_or(scene, |f| lean_pose(f, scene, q, ws, lift)))
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
    s.active && !s.plain && (matches!(s.kind, Kind::Table | Kind::Crafter) || is_furnace(s.kind) || is_surface(s.kind)) && matches!(s.phase, Phase::Enter | Phase::Inside)
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
    let zoom = f32::from(config().zoom.max(50)) / 100.0;
    live + (FOV / zoom - live) * presence(now)
}

/// How tall the black bars are, as a share of their full height: in over the first 30% of the
/// intro (the walk), out over its last 28%; the same on the way back.
pub fn bars(now: Instant) -> f32 {
    let s = state();
    if !s.config.bars || s.config.reduce_motion {
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
        Kind::Shelf => "LMB on a book \u{2014} take it, a book with writing \u{2014} read it  \u{b7}  Shift+LMB \u{2014} into the inventory  \u{b7}  Esc \u{2014} step away",
        Kind::Campfire => "Food onto a free spot of the fire  \u{b7}  click food \u{2014} take it off  \u{b7}  Esc \u{2014} stand up",
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

/// The anvil: the item lies on its face, what goes into it (a second item, a book, a material) on top.
pub const ANVIL_SPOTS: [Spot; 2] = [Spot { slot: 0, x: 0.0, z: 0.0, r: 0.2, y: 0.0 }, Spot { slot: 1, x: 0.0, z: -0.03, r: 0.14, y: 0.3 }];

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
    matches!(kind, Kind::Anvil | Kind::Smithing | Kind::Cartography | Kind::Grindstone | Kind::Brewing | Kind::Enchanting | Kind::Beacon | Kind::Campfire | Kind::Loom | Kind::Shelf | Kind::Stonecutter)
}

/// Where a station's spots are in the menu: a brewing stand is a container (300 on), the others
/// stations (600 on).
pub fn spot_base(kind: Kind) -> usize {
    if kind == Kind::Brewing { 300 } else { 600 }
}

/// Whether the worked spot gives the result (650).
pub fn has_work(kind: Kind) -> bool {
    !matches!(kind, Kind::Brewing | Kind::Enchanting | Kind::Beacon | Kind::Campfire | Kind::Shelf | Kind::Stonecutter)
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
        Kind::Anvil => &ANVIL_SPOTS,
        Kind::Brewing => &BREWING_SPOTS,
        Kind::Enchanting => &ENCHANTING_SPOTS,
        Kind::Beacon => &BEACON_SPOTS,
        Kind::Loom => &LOOM_SPOTS,
        Kind::Shelf => &SHELF_SPOTS,
        Kind::Stonecutter => &STONECUTTER_SPOTS,
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
    let out = -forward(yaw);
    if kind == Kind::Shelf {
        // ShelfStation.rest: 0.3 in front of the face's middle, 0.6 to the side, 0.25 down.
        let front = top_center(t) - DVec3::Y * 0.5 + out * 0.5;
        return front + out * 0.3 + right(yaw) * (0.6 * f) - DVec3::Y * 0.25;
    }
    if kind == Kind::Beacon {
        // BeaconStation.rest (0.86 out, 0.32 aside, 0.1 down from the top), with Hands.hidden: an
        // idle hand is down out of the view, only the reaching one comes up.
        return top_center(t) + out * 0.86 + right(yaw) * (0.32 * f) - DVec3::Y * 0.1 - DVec3::Y * 0.9 + out * 0.3;
    }
    let aside = if kind == Kind::Cartography { 0.26 } else { 0.25 };
    let low = match kind {
        Kind::Brewing => -0.85,
        Kind::Grindstone => -0.2,
        Kind::Enchanting => -0.25,
        Kind::Stonecutter => -0.4375,
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
    if kind == Kind::Anvil
        && let Some((grip, _)) = hammer_pose(t, yaw)
    {
        targets[1] = grip;
        return step_hands_to(targets, Some(1), now);
    }
    if let Some(hand) = grind_hand(t, yaw) {
        targets[1] = hand;
        return step_hands_to(targets, Some(1), now);
    }
    if let Some(s) = spot.and_then(|slot| surface_spots(kind).iter().find(|s| s.slot == slot)) {
        let side = usize::from(s.x >= -0.1);
        let lift = if carrying { 0.075 } else { 0.045 };
        targets[side] = match kind {
            // ShelfStation.handTarget: 0.07 out of the face, 0.04 below the place.
            Kind::Shelf => spot_point(t, yaw, s) - forward(yaw) * 0.07 - DVec3::Y * 0.04,
            // BeaconStation.handTarget: 0.07 towards the player, 0.04 down.
            Kind::Beacon => spot_point(t, yaw, s) - forward(yaw) * 0.07 + DVec3::Y * 0.02,
            _ => spot_point(t, yaw, s) + DVec3::Y * (0.062 + lift * 0.5) - forward(yaw) * 0.075,
        };
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
        (Kind::Anvil, 0) => "Lay the item on the anvil",
        (Kind::Anvil, _) => "What goes into it: a second item, a book or a repair material",
        (Kind::Grindstone, 0) => "Lay the item on the wheel",
        (Kind::Grindstone, _) => "A second item to grind into one",
        (Kind::Smithing, 0) => "Smithing template",
        (Kind::Smithing, 1) => "Gear: armour, a weapon or a tool",
        (Kind::Smithing, _) => "Material: an ingot or a mineral",
        (Kind::Stonecutter, _) => "Lay stone by the saw",
        (_, 0) => "A filled map goes here",
        _ => "Paper, glass or an empty map goes here",
    }
}

pub fn surface_refuse(kind: Kind) -> &'static str {
    match kind {
        Kind::Smithing => "The smithing table has no use for this",
        Kind::Anvil => "The anvil has no use for this",
        Kind::Grindstone => "The grindstone only takes enchanted or worn items",
        Kind::Brewing => "That doesn't go here",
        Kind::Enchanting => "Only lapis lazuli fits here",
        Kind::Beacon => "A beacon takes only iron, gold, emerald, diamond or netherite",
        Kind::Loom => "The loom has no use for this",
        Kind::Stonecutter => "The stonecutter has no use for this",
        _ => "The cartography table has no use for this",
    }
}

pub fn surface_work(kind: Kind) -> &'static str {
    if kind == Kind::Anvil {
        "LMB \u{2014} forge  \u{b7}  Shift \u{2014} straight into the inventory"
    } else if kind == Kind::Grindstone {
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
    rune_points(t, yaw).iter().position(|p| screen_of(*p, w, h).is_some_and(|(sx, sy)| (sx - x).hypot(sy - y) <= CIRCLE_RADIUS))
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
    POWERS.iter().position(|p| screen_of(power_point(t, yaw, p), w, h).is_some_and(|(sx, sy)| (sx - x).hypot(sy - y) <= CIRCLE_RADIUS * 0.85))
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
/// The scene's own use of a block: the use hook lets it through to the game.
pub fn set_own_use(on: bool) {
    state().own_use = on;
}

pub fn set_move_to(pos: Option<[i32; 3]>) {
    state().move_to = pos;
}

/// ShelfStation's book: it slides out of its place towards the hand, or in from the hand.
pub fn start_slide(slot: usize, inserting: bool, item: &str) {
    state().slide = Some((slot, Instant::now(), inserting, item.to_owned()));
}

pub const SLIDE_TIME: f32 = 0.35;

/// The sliding book: its item and where it is now (the place's middle, out of the face).
pub fn slide_pose(t: [i32; 3], yaw: f32) -> Option<(String, DVec3)> {
    let s = state();
    let (slot, at, inserting, item) = s.slide.clone()?;
    let age = at.elapsed().as_secs_f32();
    if age >= SLIDE_TIME {
        return None;
    }
    let spot = SHELF_SPOTS.iter().find(|p| p.slot == slot)?;
    let k = f64::from(ease(age / SLIDE_TIME));
    let out = if inserting { 1.0 - k } else { k };
    Some((item, spot_point(t, yaw, spot) - forward(yaw) * (0.02 + 0.22 * out) - DVec3::Y * (0.06 * out)))
}

pub fn take_move_to() -> Option<[i32; 3]> {
    state().move_to.take()
}

pub fn own_use() -> bool {
    state().own_use
}

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

/// The server's world: its kept tables are read once (another world's are dropped first). A file
/// that is missing is an empty list; one that exists but can't be read leaves the list unloaded, so
/// nothing is kept or taken until it can be (a later save would otherwise overwrite it).
pub fn tables_load(root: &str) {
    if root.is_empty() {
        return;
    }
    let mut t = tables();
    if t.1 && t.0 == root {
        return;
    }
    t.0 = root.to_owned();
    t.1 = false;
    t.2.clear();
    match std::fs::read_to_string(tables_file(root)) {
        Ok(text) => {
            t.2 = tables_from_text(&text);
            t.1 = true;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => t.1 = true,
        Err(_) => {}
    }
}

/// Writes `text` to `path` so a failure never leaves a half-written file: the text goes to a side
/// file that is flushed to disk, then renamed over the real one. Every error is returned.
pub fn tables_write(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let side = path.with_extension("txt.tmp");
    let done = (|| {
        let mut f = std::fs::File::create(&side)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&side, path)
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&side);
    }
    done
}

fn tables_save(root: &str, list: &[KeptTable]) -> std::io::Result<()> {
    if root.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no world folder"));
    }
    tables_write(&tables_file(root), &tables_to_text(list))
}

/// A table left with items on it: they stay there. False when this world can't keep them (no
/// folder, or the list could not be written to disk), so they go back to the player as usual and
/// nothing about the table changes. A table that still has a kept entry is never overwritten (its
/// items could not be taken off it); false then too, so neither set of items is lost.
pub fn table_keep(root: &str, dim: &str, pos: [i32; 3], yaw: f32, items: [Option<(String, u32)>; 9]) -> bool {
    if root.is_empty() {
        return false;
    }
    tables_load(root);
    let mut t = tables();
    if !t.1 || t.2.iter().any(|k| k.dim == dim && k.pos == pos) {
        return false;
    }
    if !items.iter().any(Option::is_some) {
        return true;
    }
    let mut next = t.2.clone();
    next.push(KeptTable { dim: dim.to_owned(), pos, yaw, items });
    if tables_save(root, &next).is_err() {
        return false;
    }
    t.2 = next;
    true
}

/// The items kept on a table, taken off it (the table opens with them in its grid). They are only
/// handed over once the list without them is on disk; when it can't be written they stay kept (in
/// memory and in the file alike), so no stale copy can come back after a restart.
pub fn table_take(root: &str, dim: &str, pos: [i32; 3]) -> Option<[Option<(String, u32)>; 9]> {
    if root.is_empty() {
        return None;
    }
    tables_load(root);
    let mut t = tables();
    if !t.1 {
        return None;
    }
    let i = t.2.iter().position(|k| k.dim == dim && k.pos == pos)?;
    let mut next = t.2.clone();
    let k = next.remove(i);
    if tables_save(root, &next).is_err() {
        return None;
    }
    t.2 = next;
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

// ------------------------------------------------------------------ the enchanting scene's drawing

/// A rune circle's radius on the screen, in GUI units (the jar's circles are larger than a button).
pub const CIRCLE_RADIUS: f32 = 19.0;

fn mix(a: [f32; 2], b: [f32; 2], k: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]
}

fn tint(rgb: [u8; 3], alpha: f32) -> u32 {
    u32::from_le_bytes([rgb[0], rgb[1], rgb[2], (alpha.clamp(0.0, 1.0) * 255.0) as u8])
}

/// EnchantFx.CIRCLE: the jar's rune circle on the screen, turned by `turn` radians, in `rgb`.
pub fn magic_circle(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, r: f32, turn: f32, rgb: [u8; 3], strength: f32) {
    let (s, c) = turn.sin_cos();
    let k = r * 2.0 / MAGIC_CIRCLE_SIZE as f32;
    let half = MAGIC_CIRCLE_SIZE as f32 * 0.5;
    for &(y, x, n, a) in MAGIC_CIRCLE.iter() {
        let (y0, y1) = (f32::from(y) - half, f32::from(y) + 1.0 - half);
        let (x0, x1) = (f32::from(x) - half, f32::from(x) + f32::from(n) - half);
        let p = |u: f32, v: f32| [cx + (u * c - v * s) * k, cy + (u * s + v * c) * k];
        batch.fill_quad(atlas, [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], tint(rgb, f32::from(a) / 15.0 * strength));
    }
}

/// EnchantFx.PIXEL_CIRCLE: the blue circle on the table, laid on four screen corners (near left,
/// near right, far right, far left) so it lies in the table's perspective.
pub fn pixel_circle(batch: &mut HudBatch, atlas: &HudAtlas, c: [[f32; 2]; 4], rgb: [u8; 3], strength: f32) {
    let n = PIXEL_CIRCLE_SIZE as f32;
    let at = |u: f32, v: f32| mix(mix(c[0], c[1], u), mix(c[3], c[2], u), v);
    for &(y, x, len, a) in PIXEL_CIRCLE.iter() {
        let (u0, u1) = (f32::from(x) / n, (f32::from(x) + f32::from(len)) / n);
        let (v0, v1) = (f32::from(y) / n, (f32::from(y) + 1.0) / n);
        batch.fill_quad(atlas, [at(u0, v0), at(u1, v0), at(u1, v1), at(u0, v1)], tint(rgb, f32::from(a) / 15.0 * strength));
    }
}

/// The lapis cost on a rune: a small four-pointed star.
pub fn star(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, rgb: [u8; 3]) {
    batch.fill(atlas, cx - 0.5, cy - 2.5, 1.0, 5.0, tint(rgb, 0.9));
    batch.fill(atlas, cx - 2.5, cy - 0.5, 5.0, 1.0, tint(rgb, 0.9));
    batch.fill(atlas, cx - 1.0, cy - 1.0, 2.0, 2.0, tint([255, 255, 255], 1.0));
}

/// The clue a rune whispers, in words of the game's galactic alphabet, the same for the same
/// seed and offer: the player cannot read it, it is there to look like a spell.
pub fn clue_words(seed: i32, offer: usize) -> String {
    const WORDS: [&str; 24] = [
        "til", "ar", "tifan", "uldar", "wenal", "kor", "miza", "drel", "sha", "quen", "olat", "fin", "vesk", "anta", "rhun", "ilo", "thar",
        "benu", "zeph", "cael", "mor", "usk", "ebra", "lun",
    ];
    let mut state = (i64::from(seed) ^ 0x5DEE_CE66D).wrapping_mul(6364136223846793005).wrapping_add(offer as i64 * 1442695040888963407);
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) as usize) % WORDS.len()
    };
    let count = 2 + next() % 2;
    (0..count).map(|_| WORDS[next()]).collect::<Vec<_>>().join(" ")
}

/// The enchanted-to-be item (and the crafted ones the jar floats): upright over its spot, turning.
pub fn item_upright(at: Vec3, yaw: f32, size: f32, spin: f32) -> Mat4 {
    Mat4::from_translation(at)
        * Mat4::from_rotation_y((180.0 - yaw + spin).to_radians())
        * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4)
        * Mat4::from_scale(Vec3::splat(size))
}

/// EnchantFx.itemCenter: the item floats 0.62 over the table top and bobs a little.
pub fn bob(seconds: f64) -> f64 {
    (seconds * 1.6).sin() * 0.025
}

/// The four corners (near left, near right, far right, far left) of the lapis circle on the table.
pub fn lapis_socket(t: [i32; 3], yaw: f32) -> [DVec3; 4] {
    let c = spot_point(t, yaw, &ENCHANTING_SPOTS[1]) - DVec3::Y * 0.015;
    let (r, f, k) = (right(yaw), forward(yaw), 0.115);
    [c - r * k - f * k, c + r * k - f * k, c + r * k + f * k, c - r * k + f * k]
}


// ------------------------------------------------------------------ the beacon's rings

/// The jar's ring.png across its radius (every second texel from the middle out, of 64): a soft
/// edge inside, a bright line near the rim, a quick fade outside.
const RING_PROFILE: [u8; 32] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 4, 8, 14, 21, 31, 42, 53, 74, 127, 227, 255, 255, 151, 62, 25];
/// The jar's glow.png across its radius (every second texel, of 32).
const GLOW_PROFILE: [u8; 16] = [242, 212, 182, 154, 129, 106, 85, 67, 51, 38, 26, 17, 10, 5, 1, 0];

fn band(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, r0: f32, r1: f32, color: u32) {
    const N: usize = 40;
    for i in 0..N {
        let (a0, a1) = (i as f32 / N as f32 * std::f32::consts::TAU, (i + 1) as f32 / N as f32 * std::f32::consts::TAU);
        let p = |r: f32, a: f32| [cx + r * a.cos(), cy + r * a.sin()];
        batch.fill_quad(atlas, [p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)], color);
    }
}

/// RING billboard on the screen (BeaconStation.drawPower), `r` its outer radius.
pub fn ring(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, r: f32, rgb: [u8; 3], strength: f32) {
    for (i, &a) in RING_PROFILE.iter().enumerate() {
        if a == 0 {
            continue;
        }
        let (r0, r1) = (((2 * i) as f32 - 1.0).max(0.0) / 64.0 * r, ((2 * i) as f32 + 1.0) / 64.0 * r);
        band(batch, atlas, cx, cy, r0, r1, tint(rgb, f32::from(a) / 255.0 * strength));
    }
}

/// GLOW billboard: a soft round haze, strongest in the middle.
pub fn glow(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, r: f32, rgb: [u8; 3], strength: f32) {
    for (i, &a) in GLOW_PROFILE.iter().enumerate() {
        if a == 0 {
            continue;
        }
        let (r0, r1) = ((2 * i) as f32 / 32.0 * r, (2 * i + 2) as f32 / 32.0 * r);
        band(batch, atlas, cx, cy, r0, r1, tint(rgb, f32::from(a) / 255.0 * strength));
    }
}

/// MobEffect colours of the beacon's powers, for their rings.
pub fn effect_rgb(effect: &str) -> [u8; 3] {
    match effect {
        "speed" => [0x33, 0xEB, 0xFF],
        "haste" => [0xD9, 0xC0, 0x43],
        "resistance" => [0x91, 0x46, 0xF0],
        "jump_boost" => [0xFD, 0xFF, 0x84],
        "strength" => [0xFF, 0xC7, 0x00],
        "regeneration" => [0xCD, 0x5C, 0xAB],
        _ => [220, 220, 220],
    }
}

/// The ring lying on the beacon's core (where the payment goes): world points of its bands, each
/// a quad, for the screen to project.
pub fn beacon_socket(t: [i32; 3]) -> Vec<([DVec3; 4], f32)> {
    let c = top_center(t) - DVec3::Y * 0.12;
    let r = 0.25;
    let mut out = Vec::new();
    const N: usize = 40;
    for (k, &a) in RING_PROFILE.iter().enumerate() {
        if a == 0 {
            continue;
        }
        let (r0, r1) = (((2 * k) as f64 - 1.0).max(0.0) / 64.0 * r, ((2 * k) as f64 + 1.0) / 64.0 * r);
        for i in 0..N {
            let (a0, a1) = (i as f64 / N as f64 * std::f64::consts::TAU, (i + 1) as f64 / N as f64 * std::f64::consts::TAU);
            let p = |rr: f64, aa: f64| c + DVec3::new(rr * aa.cos(), 0.0, rr * aa.sin());
            out.push(([p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)], f32::from(a) / 255.0));
        }
    }
    out
}

/// Four projected corners as a screen quad, when all four are in front.
pub fn screen_quad(q: &[DVec3; 4], w: f32, h: f32) -> Option<[[f32; 2]; 4]> {
    let mut out = [[0.0; 2]; 4];
    for (i, p) in q.iter().enumerate() {
        let (x, y) = screen_of(*p, w, h)?;
        out[i] = [x, y];
    }
    Some(out)
}

pub fn fill_screen_quad(batch: &mut HudBatch, atlas: &HudAtlas, q: [[f32; 2]; 4], rgb: [u8; 3], alpha: f32) {
    batch.fill_quad(atlas, q, tint(rgb, alpha));
}


// ------------------------------------------------------------------ moving over to the next station

/// The blocks right and left of the station in the scene (the jar's neighbouring stations).
pub fn neighbours(t: [i32; 3], yaw: f32) -> [[i32; 3]; 2] {
    let r = right(yaw);
    let (dx, dz) = (r.x.round() as i32, r.z.round() as i32);
    [[t[0] - dx, t[1], t[2] - dz], [t[0] + dx, t[1], t[2] + dz]]
}

/// A station the scene can move over to.
pub fn moves_to(kind: Kind) -> bool {
    kind == Kind::Table || is_furnace(kind) || (is_surface(kind) && kind != Kind::Campfire)
}

/// A block's eight corners: bit 0 is x, bit 1 y, bit 2 z.
pub fn block_corners(p: [i32; 3]) -> [DVec3; 8] {
    let mut out = [DVec3::ZERO; 8];
    for (i, c) in out.iter_mut().enumerate() {
        *c = DVec3::new(f64::from(p[0] + (i & 1) as i32), f64::from(p[1] + ((i >> 1) & 1) as i32), f64::from(p[2] + ((i >> 2) & 1) as i32));
    }
    out
}

/// The twelve edges of [`block_corners`].
pub const BLOCK_EDGES: [(usize, usize); 12] = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];

/// A line on the screen, `w` wide.
pub fn line(batch: &mut HudBatch, atlas: &HudAtlas, a: (f32, f32), b: (f32, f32), w: f32, color: u32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy).max(0.001);
    let (nx, ny) = (-dy / l * w * 0.5, dx / l * w * 0.5);
    batch.fill_quad(atlas, [[a.0 + nx, a.1 + ny], [b.0 + nx, b.1 + ny], [b.0 - nx, b.1 - ny], [a.0 - nx, a.1 - ny]], color);
}

pub fn set_neighbour(n: Option<[i32; 3]>) {
    state().neighbour = n;
}

pub fn neighbour() -> Option<[i32; 3]> {
    let s = state();
    if s.phase == Phase::Inside { s.neighbour } else { None }
}

pub const MOVE_OVER: &str = "RMB \u{2014} move over";


// ------------------------------------------------------------------ rituals: the light and the hammer

pub fn start_burst(kind: Kind) {
    state().burst = Some((kind, Instant::now()));
}

/// Seconds since the station's last ritual, while it still shows.
pub fn burst_age(kind: Kind) -> Option<f32> {
    let s = state();
    s.burst.filter(|b| b.0 == kind).map(|b| b.1.elapsed().as_secs_f32()).filter(|a| *a < 1.6)
}

fn spark_rand(i: u32, k: u32) -> f32 {
    let mut x = i.wrapping_mul(747_796_405).wrapping_add(k.wrapping_mul(2_891_336_453)).wrapping_add(12345);
    x ^= x >> 15;
    x = x.wrapping_mul(2_246_822_519);
    x ^= x >> 13;
    (x & 0xFFFF) as f32 / 65535.0
}

/// The light of a ritual on the screen at (cx, cy): a beam up, a glow, a ring going out and
/// sparks flying, fading over 1.4 s.
pub fn draw_burst(batch: &mut HudBatch, atlas: &HudAtlas, cx: f32, cy: f32, age: f32, rgb: [u8; 3], beam: bool) {
    let fade = (1.0 - age / 1.4).clamp(0.0, 1.0);
    let rise = (age / 0.18).min(1.0);
    // Screen flashes off: only the sparks.
    let flash = config().flashes;
    if beam && flash {
        let w = 3.0 + 9.0 * fade;
        let top = (cy - 400.0 * rise).max(-10.0);
        for (k, a) in [(1.0f32, 0.35f32), (0.45, 0.6), (0.18, 0.9)] {
            batch.fill_quad(atlas, [[cx - w * k, top], [cx + w * k, top], [cx + w * k * 1.6, cy], [cx - w * k * 1.6, cy]], tint(rgb, a * fade));
        }
    }
    if flash {
        glow(batch, atlas, cx, cy, 14.0 + 30.0 * age.min(0.6), [255, 255, 255], 0.75 * fade);
        glow(batch, atlas, cx, cy, 24.0 + 40.0 * age.min(0.8), rgb, 0.55 * fade);
        ring(batch, atlas, cx, cy, 8.0 + 60.0 * (age / 0.9).min(1.0).sqrt(), rgb, fade);
    }
    for i in 0..28u32 {
        let a = spark_rand(i, 1) * std::f32::consts::TAU;
        let v = 30.0 + 70.0 * spark_rand(i, 2);
        let t = age;
        let (sx, sy) = (cx + a.cos() * v * t, cy + a.sin() * v * t * 0.6 - 40.0 * t + 45.0 * t * t);
        let c = match i % 3 {
            0 => rgb,
            1 => [255, 255, 255],
            _ => [140, 230, 255],
        };
        let size = 1.0 + spark_rand(i, 3);
        batch.fill(atlas, sx - size * 0.5, sy - size * 0.5, size, size, tint(c, fade));
    }
}

/// The forge's clock (s): the anvil's hammer comes in from beside the camera and strikes three
/// times (the result is taken at the last strike); the grindstone's hand rubs the item on the
/// wheel and lifts it off.
pub const FORGE_HIT: f32 = 1.2;
pub const FORGE_TIME: f32 = 1.6;
pub const ANVIL_HITS: [f32; 3] = [0.6, 0.9, 1.2];
pub const GRIND_TAKE: f32 = 1.1;
pub const GRIND_TIME: f32 = 1.4;

fn forge_len(kind: Kind) -> f32 {
    if kind == Kind::Grindstone { GRIND_TIME } else { FORGE_TIME }
}

fn forge_age() -> Option<f32> {
    let s = state();
    let f = s.forge?;
    let age = f.0.elapsed().as_secs_f32();
    (age < forge_len(s.kind)).then_some(age)
}

/// LMB on the anvil's or the grindstone's result: the work is shown, the result taken in it.
pub fn start_forge(shift: bool) {
    let mut s = state();
    let len = forge_len(s.kind);
    if s.forge.is_some_and(|f| f.0.elapsed().as_secs_f32() < len) {
        return;
    }
    let now = Instant::now();
    s.forge = Some((now, shift, false));
    s.burst = Some((s.kind, now));
}

/// The strike (or the lift off the wheel) has come: take the result now (with Shift or not), once.
pub fn forge_due() -> Option<bool> {
    let mut s = state();
    let take = if s.kind == Kind::Grindstone { GRIND_TAKE } else { FORGE_HIT };
    let f = s.forge.as_mut()?;
    if f.2 || f.0.elapsed().as_secs_f32() < take {
        return None;
    }
    f.2 = true;
    Some(f.1)
}

pub fn forging() -> bool {
    forge_age().is_some()
}

/// Seconds since the last spark shower: the anvil's last strike, the grindstone's last turn.
pub fn forge_spark_age() -> Option<f32> {
    let kind = state().kind;
    let age = forge_age()?;
    if kind == Kind::Grindstone {
        return (age > 0.15 && age < GRIND_TAKE).then(|| (age - 0.15) % 0.3);
    }
    ANVIL_HITS.iter().rev().find(|h| age >= **h).map(|h| age - h)
}

/// The grindstone's hand in the work: on the item, rubbing it to and fro, then up off the wheel.
pub fn grind_hand(t: [i32; 3], yaw: f32) -> Option<DVec3> {
    if state().kind != Kind::Grindstone {
        return None;
    }
    let age = forge_age()?;
    let item = spot_point(t, yaw, &GRINDSTONE_SPOTS[0]) + DVec3::Y * 0.06 - forward(yaw) * 0.04;
    if age < GRIND_TAKE {
        let rub = f64::from((age * 17.0).sin()) * 0.07 * f64::from(window(age, 0.0, 0.15));
        return Some(item + forward(yaw) * rub + DVec3::Y * (rub.abs() * 0.3));
    }
    let k = f64::from(ease(window(age, GRIND_TAKE, GRIND_TIME)));
    Some(item + DVec3::Y * (0.18 * k) - forward(yaw) * (0.12 * k))
}

/// The hammer in the work: the grip and the handle's angle from straight up, towards the item.
/// It comes in big from beside the camera, is raised, strikes three times with a bounce after
/// each, and goes back out the way it came.
pub fn hammer_pose(t: [i32; 3], yaw: f32) -> Option<(DVec3, f64)> {
    if state().kind != Kind::Anvil {
        return None;
    }
    let age = forge_age()?;
    let item = spot_point(t, yaw, &ANVIL_SPOTS[0]);
    let rest = item + right(yaw) * 0.46 - forward(yaw) * 0.08 + DVec3::Y * 0.06;
    let near = item + right(yaw) * 0.38 - forward(yaw) * 0.62 + DVec3::Y * 0.42;
    let raised = rest + DVec3::Y * 0.12;
    let (grip, deg) = if age < 0.4 {
        let k = f64::from(ease(age / 0.4));
        (near.lerp(raised, k), -40.0 + 15.0 * k)
    } else if age < 1.3 {
        let local = (age - 0.4) % 0.3;
        if local < 0.2 {
            let s = f64::from(local / 0.2);
            (rest + DVec3::Y * (0.12 * (1.0 - s)), -25.0 + 113.0 * s * s)
        } else {
            let b = f64::from(ease((local - 0.2) / 0.1));
            (rest + DVec3::Y * (0.12 * b), 88.0 - 113.0 * b)
        }
    } else {
        let k = f64::from(ease(window(age, 1.3, FORGE_TIME)));
        (raised.lerp(near, k), -25.0 - 15.0 * k)
    };
    Some((grip, deg.to_radians()))
}

/// The jar's hammer (HammerModel, from hammer.json): handle, head, pommel and collar as boxes, in
/// the colours of their textures, at the swing's pose; quads with their shaded colour.
pub fn hammer_quads(t: [i32; 3], yaw: f32) -> Vec<([DVec3; 4], u32)> {
    let Some((grip, angle)) = hammer_pose(t, yaw) else {
        return Vec::new();
    };
    // Model units (1/16) to the world: the hammer is about 0.5 long.
    let k = 0.5 / 18.0;
    let (r, f) = (right(yaw), forward(yaw));
    // The handle leans from straight up towards the item (the left) by `angle`.
    let up = DVec3::Y * angle.cos() - r * angle.sin();
    let side = r * angle.cos() + DVec3::Y * angle.sin();
    let at = |x: f64, y: f64, z: f64| grip + side * ((x - 8.0) * k) + up * ((y - 1.0) * k) + f * ((z - 8.0) * k);
    let boxes: [([f64; 3], [f64; 3], [u8; 3]); 4] = [
        ([7.125, 1.0, 7.125], [8.875, 11.0, 8.875], [118, 84, 54]),
        ([2.0, 11.0, 4.0], [14.0, 18.0, 12.0], [64, 62, 66]),
        ([7.0, 0.5, 7.0], [9.0, 2.0, 9.0], [150, 150, 142]),
        ([7.0, 10.0, 7.0], [9.0, 11.0, 9.0], [134, 153, 148]),
    ];
    let mut out = Vec::new();
    for (a, b, rgb) in boxes {
        let c = |i: usize| at(if i & 1 == 0 { a[0] } else { b[0] }, if i & 2 == 0 { a[1] } else { b[1] }, if i & 4 == 0 { a[2] } else { b[2] });
        for (face, shade) in [([0, 1, 3, 2], 0.8), ([4, 5, 7, 6], 0.8), ([0, 1, 5, 4], 0.5), ([2, 3, 7, 6], 1.0), ([0, 2, 6, 4], 0.6), ([1, 3, 7, 5], 0.6)] {
            let col = u32::from_le_bytes([(f64::from(rgb[0]) * shade) as u8, (f64::from(rgb[1]) * shade) as u8, (f64::from(rgb[2]) * shade) as u8, 255]);
            out.push(([c(face[0]), c(face[1]), c(face[2]), c(face[3])], col));
        }
    }
    out
}


// FX-DATA-BEGIN (made by tools/bake_fx.py from the jar's magic_circle.png and enchant_circle_pixel.png)
pub const MAGIC_CIRCLE_SIZE: usize = 96;
pub static MAGIC_CIRCLE: [(u8, u8, u8, u8); 4099] = [(1,39,18,1), (2,35,4,1), (2,39,1,2), (2,40,1,3), (2,41,1,5), (2,42,1,7), (2,43,1,8), (2,44,1,9), (2,45,1,10), (2,46,4,11), (2,50,1,10), (2,51,1,9), (2,52,1,8), (2,53,1,7), (2,54,1,5), (2,55,1,4), (2,56,1,2), (2,57,4,1), (3,32,2,1), (3,34,1,2), (3,35,1,3), (3,36,1,7), (3,37,1,9), (3,38,1,12), (3,39,1,14), (3,40,1,15), (3,41,1,14), (3,42,1,12), (3,43,1,11), (3,44,1,10), (3,45,1,13), (3,46,4,9), (3,50,1,13), (3,51,1,10), (3,52,1,11), (3,53,1,12), (3,54,1,14), (3,55,1,15), (3,56,1,14), (3,57,1,12), (3,58,1,10), (3,59,1,7), (3,60,1,3), (3,61,1,2), (3,62,2,1), (4,29,3,1), (4,32,1,2), (4,33,1,7), (4,34,1,12), (4,35,2,15), (4,37,1,14), (4,38,1,11), (4,39,1,7), (4,40,1,13), (4,41,1,4), (4,42,3,2), (4,45,1,5), (4,46,4,2), (4,50,1,5), (4,51,3,2), (4,54,1,4), (4,55,1,13), (4,56,1,6), (4,57,1,10), (4,58,1,14), (4,59,2,15), (4,61,1,12), (4,62,1,7), (4,63,1,2), (4,64,3,1), (5,27,2,1), (5,29,1,2), (5,30,1,5), (5,31,1,10), (5,32,1,13), (5,33,1,14), (5,34,1,11), (5,35,1,12), (5,36,1,6), (5,37,6,2), (5,43,1,3), (5,44,1,4), (5,45,1,5), (5,46,4,6), (5,50,1,5), (5,51,1,4), (5,52,1,3), (5,53,2,2), (5,55,1,3), (5,56,3,2), (5,59,1,5), (5,60,1,12), (5,61,1,11), (5,62,2,14), (5,64,1,10), (5,65,1,6), (5,66,1,2), (5,67,2,1), (6,25,2,1), (6,27,1,3), (6,28,1,8), (6,29,1,12), (6,30,1,15), (6,31,1,12), (6,32,1,7), (6,33,1,3), (6,34,3,2), (6,37,1,4), (6,38,1,6), (6,39,3,8), (6,42,1,7), (6,43,1,6), (6,44,2,5), (6,46,1,7), (6,47,1,5), (6,48,3,4), (6,51,1,5), (6,52,1,6), (6,53,1,7), (6,54,3,8), (6,57,1,6), (6,58,1,4), (6,59,3,2), (6,62,1,3), (6,63,1,6), (6,64,1,12), (6,65,1,15), (6,66,1,13), (6,67,1,8), (6,68,1,3), (6,69,2,1), (7,24,1,1), (7,25,1,2), (7,26,1,6), (7,27,1,14), (7,28,1,15), (7,29,1,11), (7,30,1,6), (7,31,1,7), (7,32,3,2), (7,35,1,7), (7,36,1,11), (7,37,1,8), (7,38,1,4), (7,39,6,1), (7,45,1,2), (7,46,1,13), (7,47,1,5), (7,48,1,3), (7,49,1,7), (7,50,5,1), (7,55,2,2), (7,57,1,5), (7,58,1,8), (7,59,1,11), (7,60,1,7), (7,61,1,3), (7,62,2,2), (7,64,1,7), (7,65,1,6), (7,66,1,10), (7,67,1,15), (7,68,1,14), (7,69,1,7), (7,70,1,2), (7,71,1,1), (8,22,2,1), (8,24,1,5), (8,25,1,11), (8,26,1,15), (8,27,1,10), (8,28,1,5), (8,29,3,2), (8,32,1,6), (8,33,1,8), (8,34,1,7), (8,35,1,4), (8,36,4,1), (8,45,1,1), (8,46,2,2), (8,48,1,7), (8,49,1,5), (8,50,1,1), (8,53,2,1), (8,55,1,11), (8,56,2,9), (8,58,1,4), (8,59,1,1), (8,60,1,4), (8,61,1,7), (8,62,1,8), (8,63,1,6), (8,64,1,3), (8,65,2,2), (8,67,1,4), (8,68,1,10), (8,69,1,15), (8,70,1,12), (8,71,1,6), (8,72,2,1), (9,20,2,1), (9,22,1,3), (9,23,1,10), (9,24,1,14), (9,25,1,10), (9,26,1,9), (9,27,1,3), (9,28,1,2), (9,29,1,4), (9,30,2,8), (9,32,1,4), (9,33,2,1), (9,46,1,4), (9,47,1,7), (9,48,1,10), (9,49,1,2), (9,50,1,1), (9,54,1,3), (9,55,1,14), (9,56,1,7), (9,57,1,6), (9,58,2,1), (9,61,2,1), (9,63,1,3), (9,64,1,8), (9,65,1,9), (9,66,1,4), (9,67,1,2), (9,68,1,3), (9,69,1,9), (9,70,1,10), (9,71,1,14), (9,72,1,11), (9,73,1,4), (9,74,2,1), (10,19,2,1), (10,21,1,6), (10,22,2,14), (10,24,1,5), (10,25,2,2), (10,27,1,3), (10,28,1,10), (10,29,1,9), (10,30,1,4), (10,31,1,6), (10,32,2,1), (10,38,1,1), (10,39,1,6), (10,40,1,1), (10,46,1,1), (10,47,1,6), (10,48,1,10), (10,49,2,1), (10,53,1,1), (10,54,1,4), (10,55,1,12), (10,56,1,2), (10,57,1,8), (10,58,1,1), (10,62,1,1), (10,63,1,3), (10,64,1,13), (10,65,1,3), (10,66,1,8), (10,67,1,10), (10,68,1,3), (10,69,2,2), (10,71,1,5), (10,72,1,14), (10,73,1,15), (10,74,1,7), (10,75,2,1), (11,18,1,1), (11,19,1,2), (11,20,1,7), (11,21,2,14), (11,23,1,4), (11,24,2,2), (11,26,1,6), (11,27,1,8), (11,28,1,3), (11,29,1,4), (11,30,1,8), (11,31,1,4), (11,32,1,1), (11,37,1,1), (11,38,2,5), (11,40,1,2), (11,41,1,1), (11,46,1,1), (11,47,1,3), (11,48,1,8), (11,49,1,4), (11,50,1,1), (11,53,1,1), (11,54,2,6), (11,56,2,1), (11,63,1,1), (11,64,1,6), (11,65,2,2), (11,67,1,3), (11,68,2,7), (11,70,2,2), (11,72,1,4), (11,73,1,13), (11,74,1,15), (11,75,1,8), (11,76,1,2), (11,77,1,1), (12,17,1,1), (12,18,1,4), (12,19,1,12), (12,20,1,14), (12,21,1,6), (12,22,1,7), (12,23,1,2), (12,24,1,6), (12,25,1,8), (12,26,1,3), (12,27,2,1), (12,29,1,7), (12,30,1,14), (12,31,1,11), (12,32,1,8), (12,33,1,1), (12,37,1,1), (12,38,1,5), (12,39,1,4), (12,40,1,7), (12,41,1,3), (12,42,4,1), (12,46,4,2), (12,50,4,1), (12,54,1,2), (12,55,2,1), (12,63,2,1), (12,65,1,4), (12,66,1,7), (12,67,2,1), (12,69,1,3), (12,70,1,8), (12,71,1,6), (12,72,1,2), (12,73,1,6), (12,74,1,5), (12,75,2,13), (12,77,1,5), (12,78,1,1), (13,16,1,1), (13,17,1,3), (13,18,2,14), (13,20,1,5), (13,21,2,2), (13,23,1,8), (13,24,1,6), (13,25,2,1), (13,28,2,1), (13,30,1,4), (13,31,2,7), (13,33,1,1), (13,36,3,1), (13,39,2,2), (13,41,1,4), (13,42,1,7), (13,43,1,9), (13,44,1,11), (13,45,1,13), (13,46,4,14), (13,50,1,13), (13,51,1,11), (13,52,1,9), (13,53,1,7), (13,54,1,4), (13,55,1,2), (13,56,3,1), (13,63,1,1), (13,64,1,3), (13,65,1,11), (13,66,1,2), (13,67,1,1), (13,69,2,1), (13,71,1,5), (13,72,1,8), (13,73,2,2), (13,75,1,4), (13,76,1,13), (13,77,1,14), (13,78,1,4), (13,79,1,1), (14,15,1,1), (14,16,1,4), (14,17,2,13), (14,19,1,4), (14,20,1,2), (14,21,1,3), (14,22,1,9), (14,23,1,5), (14,24,2,1), (14,29,1,1), (14,30,1,2), (14,31,1,11), (14,32,1,2), (14,33,4,1), (14,37,1,4), (14,38,1,7), (14,39,2,10), (14,41,1,9), (14,42,1,7), (14,43,1,5), (14,44,1,4), (14,45,1,3), (14,46,1,6), (14,47,2,10), (14,49,1,6), (14,50,1,3), (14,51,1,4), (14,52,1,5), (14,53,1,7), (14,54,1,9), (14,55,2,10), (14,57,1,8), (14,58,1,4), (14,59,1,2), (14,60,3,1), (14,63,1,3), (14,64,2,10), (14,66,1,2), (14,71,1,1), (14,72,1,4), (14,73,1,9), (14,74,1,4), (14,75,1,2), (14,76,1,3), (14,77,1,12), (14,78,1,13), (14,79,1,5), (14,80,1,1), (15,14,1,1), (15,15,1,6), (15,16,1,14), (15,17,1,11), (15,18,1,6), (15,19,1,2), (15,20,1,5), (15,21,1,7), (15,22,1,4), (15,23,1,10), (15,24,1,1), (15,30,1,3), (15,31,1,5), (15,32,1,1), (15,33,1,2), (15,34,1,4), (15,35,1,9), (15,36,1,10), (15,37,1,8), (15,38,1,5), (15,39,1,3), (15,40,1,5), (15,41,4,6), (15,45,1,7), (15,46,1,12), (15,47,2,7), (15,49,1,11), (15,50,1,8), (15,51,4,6), (15,55,1,5), (15,56,1,3), (15,57,1,4), (15,58,1,8), (15,59,1,10), (15,60,1,9), (15,61,1,5), (15,62,2,2), (15,64,1,3), (15,65,1,5), (15,66,1,1), (15,72,2,1), (15,74,1,6), (15,75,1,5), (15,76,1,2), (15,77,1,6), (15,78,1,10), (15,79,1,15), (15,80,1,7), (15,81,2,1), (16,13,1,1), (16,14,1,4), (16,15,1,14), (16,16,1,11), (16,17,2,2), (16,19,1,5), (16,20,1,8), (16,21,1,2), (16,22,1,3), (16,23,1,10), (16,24,2,1), (16,30,1,1), (16,31,1,2), (16,32,1,3), (16,33,2,11), (16,35,1,5), (16,36,1,2), (16,37,1,4), (16,38,1,8), (16,39,1,7), (16,40,1,4), (16,41,4,1), (16,45,1,4), (16,46,1,10), (16,47,1,9), (16,48,1,10), (16,49,1,8), (16,50,1,5), (16,51,4,1), (16,55,1,4), (16,56,1,7), (16,57,1,8), (16,58,1,5), (16,59,1,2), (16,60,1,4), (16,61,1,10), (16,62,1,12), (16,63,1,4), (16,64,1,2), (16,65,2,1), (16,70,1,1), (16,73,2,1), (16,75,1,7), (16,76,1,6), (16,77,2,2), (16,79,1,9), (16,80,1,15), (16,81,1,5), (16,82,1,1), (17,12,1,1), (17,13,1,3), (17,14,1,13), (17,15,1,11), (17,16,2,2), (17,18,1,6), (17,19,1,7), (17,20,2,1), (17,22,1,2), (17,23,1,9), (17,24,1,2), (17,25,2,1), (17,28,2,1), (17,30,1,4), (17,31,1,9), (17,32,1,10), (17,33,1,4), (17,34,1,3), (17,35,2,6), (17,37,1,4), (17,38,3,1), (17,44,1,1), (17,45,1,2), (17,46,1,10), (17,47,1,13), (17,48,1,12), (17,49,1,10), (17,50,1,2), (17,51,1,1), (17,55,3,1), (17,58,1,4), (17,59,2,6), (17,61,2,4), (17,63,1,9), (17,64,1,10), (17,65,1,4), (17,66,4,1), (17,70,1,9), (17,71,1,2), (17,75,1,1), (17,76,2,6), (17,78,2,2), (17,80,1,10), (17,81,1,13), (17,82,1,4), (17,83,1,1), (18,11,1,1), (18,12,1,4), (18,13,1,14), (18,14,1,13), (18,15,1,6), (18,16,1,2), (18,17,1,6), (18,18,1,5), (18,19,1,1), (18,21,1,1), (18,22,1,3), (18,23,1,10), (18,24,1,9), (18,25,1,11), (18,26,1,5), (18,27,1,1), (18,28,1,2), (18,29,1,8), (18,30,1,9), (18,31,1,3), (18,32,1,4), (18,33,1,6), (18,34,1,5), (18,35,2,1), (18,44,1,1), (18,45,1,2), (18,46,1,12), (18,47,2,6), (18,49,1,11), (18,50,1,2), (18,51,1,1), (18,59,2,1), (18,61,1,4), (18,62,1,6), (18,63,1,4), (18,64,1,3), (18,65,2,9), (18,67,1,2), (18,68,1,1), (18,69,1,6), (18,70,1,7), (18,71,1,1), (18,76,1,1), (18,77,1,4), (18,78,1,6), (18,79,1,2), (18,80,1,6), (18,81,1,12), (18,82,1,14), (18,83,1,5), (18,84,1,1), (19,10,1,1), (19,11,1,2), (19,12,1,12), (19,13,1,14), (19,14,1,4), (19,15,1,2), (19,16,1,5), (19,17,1,7), (19,18,1,1), (19,22,1,1), (19,23,1,2), (19,24,2,8), (19,26,1,2), (19,27,1,4), (19,28,1,13), (19,29,1,6), (19,30,1,2), (19,31,1,8), (19,32,1,5), (19,33,2,1), (19,44,1,1), (19,45,1,6), (19,46,1,9), (19,47,2,2), (19,49,2,6), (19,51,1,1), (19,61,2,1), (19,63,1,4), (19,64,1,8), (19,65,1,3), (19,66,1,5), (19,67,1,13), (19,68,1,5), (19,69,1,2), (19,70,1,7), (19,71,1,1), (19,77,1,1), (19,78,2,6), (19,80,1,2), (19,81,1,3), (19,82,2,13), (19,84,1,2), (19,85,1,1), (20,9,2,1), (20,11,1,7), (20,12,1,14), (20,13,1,5), (20,14,1,2), (20,15,1,5), (20,16,1,8), (20,17,1,2), (20,18,1,1), (20,23,1,1), (20,24,1,5), (20,25,1,2), (20,26,1,6), (20,27,1,10), (20,28,1,3), (20,29,1,4), (20,30,1,6), (20,31,1,2), (20,32,1,1), (20,44,1,1), (20,45,1,10), (20,46,1,2), (20,47,3,1), (20,50,1,9), (20,51,1,1), (20,63,2,1), (20,65,1,6), (20,66,1,5), (20,67,1,3), (20,68,1,10), (20,69,1,7), (20,70,1,2), (20,71,1,1), (20,78,1,1), (20,79,1,7), (20,80,1,6), (20,81,1,2), (20,82,1,4), (20,83,1,13), (20,84,1,8), (20,85,2,1), (21,9,1,1), (21,10,1,6), (21,11,1,14), (21,12,1,6), (21,13,1,2), (21,14,1,3), (21,15,1,8), (21,16,1,6), (21,17,1,7), (21,18,2,1), (21,23,1,1), (21,24,1,2), (21,25,1,10), (21,26,1,8), (21,27,1,3), (21,28,1,7), (21,29,1,3), (21,30,1,1), (21,43,1,1), (21,44,1,7), (21,45,1,5), (21,46,1,1), (21,49,1,1), (21,50,1,4), (21,51,1,7), (21,52,1,1), (21,65,1,1), (21,66,1,3), (21,67,1,7), (21,68,1,3), (21,69,1,7), (21,70,1,10), (21,71,1,3), (21,72,1,1), (21,77,1,1), (21,78,1,4), (21,79,1,3), (21,80,1,7), (21,81,1,4), (21,82,1,2), (21,83,1,5), (21,84,1,15), (21,85,1,7), (21,86,1,1), (22,8,1,1), (22,9,1,3), (22,10,2,14), (22,12,1,6), (22,13,1,2), (22,14,1,9), (22,15,1,2), (22,16,1,10), (22,17,1,5), (22,18,1,8), (22,19,2,1), (22,22,2,1), (22,24,1,11), (22,25,1,6), (22,26,1,3), (22,27,1,7), (22,28,2,1), (22,42,1,1), (22,43,1,2), (22,44,1,11), (22,45,1,1), (22,50,1,1), (22,51,1,9), (22,52,1,2), (22,53,1,1), (22,66,2,1), (22,68,1,7), (22,69,1,3), (22,70,1,5), (22,71,1,11), (22,72,1,2), (22,73,1,1), (22,76,2,1), (22,78,1,9), (22,79,2,2), (22,81,1,9), (22,82,1,2), (22,83,1,6), (22,84,1,13), (22,85,1,15), (22,86,1,4), (22,87,1,1), (23,8,1,1), (23,9,1,10), (23,10,1,14), (23,11,1,4), (23,12,1,2), (23,13,1,7), (23,14,1,5), (23,15,2,1), (23,17,1,2), (23,18,1,6), (23,19,1,9), (23,20,1,3), (23,21,2,1), (23,23,1,9), (23,24,1,6), (23,25,1,3), (23,26,1,6), (23,27,1,1), (23,42,1,1), (23,43,1,9), (23,44,1,6), (23,45,1,1), (23,50,1,1), (23,51,1,4), (23,52,1,9), (23,53,1,1), (23,68,1,1), (23,69,1,6), (23,70,1,4), (23,71,1,5), (23,72,1,10), (23,73,1,2), (23,74,1,1), (23,75,1,4), (23,76,2,2), (23,78,1,11), (23,79,1,7), (23,80,1,6), (23,81,1,4), (23,82,1,9), (23,83,1,2), (23,84,1,4), (23,85,1,13), (23,86,1,11), (23,87,1,1), (24,7,1,1), (24,8,1,5), (24,9,1,14), (24,10,1,5), (24,11,1,2), (24,12,2,6), (24,14,1,1), (24,16,1,1), (24,17,1,5), (24,18,1,7), (24,19,1,10), (24,20,1,6), (24,21,1,3), (24,22,1,11), (24,23,1,6), (24,24,1,5), (24,25,1,6), (24,26,1,1), (24,41,1,1), (24,42,1,5), (24,43,1,10), (24,44,1,1), (24,51,1,1), (24,52,1,7), (24,53,1,5), (24,54,1,1), (24,69,1,1), (24,70,3,5), (24,73,1,11), (24,74,1,3), (24,75,1,8), (24,76,1,9), (24,77,1,1), (24,78,1,3), (24,79,1,2), (24,80,2,1), (24,82,1,4), (24,83,1,7), (24,84,1,2), (24,85,1,4), (24,86,1,14), (24,87,1,6), (24,88,1,1), (25,6,1,1), (25,7,1,2), (25,8,1,11), (25,9,1,10), (25,10,2,2), (25,12,1,8), (25,13,1,1), (25,17,1,2), (25,18,2,13), (25,20,1,3), (25,21,1,10), (25,22,1,6), (25,23,1,3), (25,24,1,6), (25,25,1,1), (25,41,1,1), (25,42,1,11), (25,43,1,2), (25,44,1,1), (25,51,2,1), (25,53,1,10), (25,54,1,1), (25,70,1,1), (25,71,1,5), (25,72,1,4), (25,73,1,5), (25,74,1,11), (25,75,1,2), (25,76,1,8), (25,77,1,5), (25,78,1,1), (25,81,2,1), (25,83,1,8), (25,84,2,2), (25,86,1,9), (25,87,1,12), (25,88,1,2), (25,89,1,1), (26,6,1,1), (26,7,1,6), (26,8,1,15), (26,9,1,9), (26,10,1,2), (26,11,1,6), (26,12,1,3), (26,13,1,1), (26,17,1,1), (26,18,1,5), (26,19,1,2), (26,20,1,6), (26,21,1,8), (26,22,1,3), (26,23,1,6), (26,24,1,1), (26,40,1,1), (26,41,1,5), (26,42,1,7), (26,43,1,1), (26,52,1,1), (26,53,1,6), (26,54,1,5), (26,55,1,1), (26,71,1,1), (26,72,1,6), (26,73,1,3), (26,74,2,7), (26,76,1,2), (26,77,1,4), (26,82,1,1), (26,83,1,3), (26,84,1,7), (26,85,1,2), (26,86,1,9), (26,87,1,14), (26,88,1,7), (26,89,1,1), (27,5,1,1), (27,6,1,3), (27,7,1,14), (27,8,1,10), (27,9,2,3), (27,11,1,8), (27,12,1,1), (27,18,1,1), (27,19,1,4), (27,20,1,10), (27,21,1,3), (27,22,1,7), (27,23,1,1), (27,39,1,1), (27,40,1,2), (27,41,1,10), (27,42,1,2), (27,43,1,1), (27,53,1,1), (27,54,1,9), (27,55,1,2), (27,56,1,1), (27,72,1,1), (27,73,1,7), (27,74,1,4), (27,75,1,10), (27,76,1,5), (27,77,1,1), (27,83,1,1), (27,84,1,7), (27,85,1,4), (27,86,1,3), (27,87,1,9), (27,88,1,15), (27,89,1,4), (27,90,1,1), (28,5,1,1), (28,6,1,8), (28,7,1,15), (28,8,1,5), (28,9,1,2), (28,10,1,9), (28,11,1,3), (28,12,2,1), (28,17,1,1), (28,18,1,2), (28,19,1,13), (28,20,1,4), (28,21,1,7), (28,22,2,1), (28,39,1,1), (28,40,1,8), (28,41,1,7), (28,42,1,1), (28,53,1,1), (28,54,1,5), (28,55,1,8), (28,56,1,1), (28,73,1,1), (28,74,1,7), (28,75,1,3), (28,76,1,13), (28,77,1,3), (28,78,1,1), (28,83,1,1), (28,84,1,2), (28,85,1,11), (28,86,1,2), (28,87,1,4), (28,88,1,15), (28,89,1,9), (28,90,1,1), (29,4,1,1), (29,5,1,2), (29,6,1,12), (29,7,1,11), (29,8,1,2), (29,9,1,4), (29,10,1,8), (29,11,1,2), (29,12,1,9), (29,13,1,5), (29,14,1,1), (29,17,1,1), (29,18,1,9), (29,19,1,7), (29,20,1,5), (29,21,1,4), (29,22,1,1), (29,38,1,1), (29,39,1,4), (29,40,1,12), (29,41,1,2), (29,42,12,1), (29,54,1,2), (29,55,1,10), (29,56,1,4), (29,57,1,1), (29,73,1,1), (29,74,1,3), (29,75,2,6), (29,77,1,9), (29,78,2,1), (29,83,1,2), (29,84,2,7), (29,86,1,5), (29,87,1,2), (29,88,1,10), (29,89,1,13), (29,90,1,2), (29,91,1,1), (30,4,1,1), (30,5,1,5), (30,6,1,15), (30,7,1,6), (30,8,1,2), (30,9,1,8), (30,10,3,1), (30,13,1,4), (30,14,3,1), (30,17,1,4), (30,18,1,12), (30,19,1,10), (30,20,1,11), (30,21,1,9), (30,22,1,2), (30,23,15,1), (30,38,1,2), (30,39,1,10), (30,40,1,4), (30,41,1,2), (30,42,1,3), (30,43,1,6), (30,44,3,8), (30,47,1,9), (30,48,1,10), (30,49,3,8), (30,52,1,6), (30,53,1,3), (30,54,1,2), (30,55,1,3), (30,56,1,10), (30,57,1,2), (30,58,15,1), (30,73,1,2), (30,74,1,9), (30,75,1,10), (30,76,2,11), (30,78,1,5), (30,79,1,1), (30,80,1,3), (30,81,1,2), (30,82,2,1), (30,84,1,9), (30,85,1,3), (30,86,1,9), (30,87,1,2), (30,88,1,5), (30,89,1,14), (30,90,1,6), (30,91,1,1), (31,4,1,1), (31,5,1,10), (31,6,1,12), (31,7,1,7), (31,8,1,2), (31,9,1,8), (31,10,3,1), (31,13,1,11), (31,14,1,7), (31,15,1,1), (31,16,1,2), (31,17,1,10), (31,18,1,11), (31,19,1,9), (31,20,1,5), (31,21,1,8), (31,22,1,7), (31,23,15,3), (31,38,1,6), (31,39,1,10), (31,40,1,4), (31,41,1,9), (31,42,1,12), (31,43,1,7), (31,44,3,4), (31,47,1,6), (31,48,1,7), (31,49,3,4), (31,52,1,6), (31,53,1,11), (31,54,1,10), (31,55,1,4), (31,56,1,9), (31,57,1,6), (31,58,15,3), (31,73,2,7), (31,75,1,4), (31,76,2,10), (31,78,1,11), (31,79,1,2), (31,80,1,10), (31,81,1,12), (31,82,1,5), (31,83,1,1), (31,84,1,4), (31,85,1,5), (31,86,1,7), (31,87,1,3), (31,88,1,7), (31,89,2,11), (31,91,1,2), (31,92,1,1), (32,3,1,1), (32,4,1,2), (32,5,1,13), (32,6,1,7), (32,7,1,2), (32,8,1,6), (32,9,1,3), (32,10,1,1), (32,13,1,1), (32,14,1,4), (32,15,1,2), (32,16,1,3), (32,17,2,10), (32,19,1,7), (32,20,2,11), (32,22,1,10), (32,23,1,7), (32,24,12,6), (32,36,2,7), (32,38,1,14), (32,39,1,13), (32,40,1,9), (32,41,6,7), (32,47,2,9), (32,49,6,7), (32,55,1,8), (32,56,2,13), (32,58,2,7), (32,60,12,6), (32,72,1,7), (32,73,1,10), (32,74,2,11), (32,76,1,6), (32,77,1,11), (32,78,1,10), (32,79,1,5), (32,80,1,3), (32,81,1,5), (32,82,1,6), (32,83,1,5), (32,84,2,1), (32,86,1,3), (32,87,1,7), (32,88,1,2), (32,89,1,6), (32,90,1,14), (32,91,1,3), (32,92,1,1), (33,3,1,1), (33,4,1,7), (33,5,1,14), (33,6,1,3), (33,7,1,2), (33,8,1,8), (33,9,1,1), (33,14,2,1), (33,16,1,11), (33,17,1,5), (33,18,1,8), (33,19,2,10), (33,21,1,13), (33,22,1,3), (33,23,13,1), (33,36,1,2), (33,37,1,9), (33,38,1,10), (33,39,1,3), (33,40,7,1), (33,47,1,4), (33,48,1,5), (33,49,7,1), (33,56,1,2), (33,57,2,9), (33,59,1,3), (33,60,13,1), (33,73,1,3), (33,74,1,12), (33,75,1,9), (33,76,1,10), (33,77,1,9), (33,78,1,4), (33,79,1,12), (33,80,1,3), (33,81,1,10), (33,82,2,1), (33,86,1,1), (33,87,1,8), (33,88,2,2), (33,90,1,14), (33,91,1,8), (33,92,1,1), (34,3,1,2), (34,4,1,12), (34,5,1,11), (34,6,2,2), (34,8,1,7), (34,9,1,1), (34,14,1,1), (34,15,1,4), (34,16,1,11), (34,17,1,3), (34,18,1,5), (34,19,1,2), (34,20,1,3), (34,21,1,10), (34,22,1,4), (34,23,1,1), (34,34,1,1), (34,35,1,2), (34,36,1,10), (34,37,1,14), (34,38,1,3), (34,39,1,1), (34,43,4,1), (34,47,1,4), (34,48,1,5), (34,49,4,1), (34,56,1,1), (34,57,1,2), (34,58,1,12), (34,59,1,10), (34,60,2,1), (34,72,1,1), (34,73,1,3), (34,74,1,9), (34,75,1,3), (34,76,1,2), (34,77,2,4), (34,79,1,9), (34,80,1,6), (34,81,2,1), (34,86,1,1), (34,87,1,6), (34,88,1,3), (34,89,1,2), (34,90,1,10), (34,91,1,13), (34,92,1,2), (35,2,1,1), (35,3,1,3), (35,4,1,15), (35,5,1,12), (35,6,1,2), (35,7,1,7), (35,8,1,4), (35,14,1,1), (35,15,1,8), (35,16,1,5), (35,17,1,6), (35,18,3,1), (35,21,1,4), (35,22,1,11), (35,23,1,2), (35,34,1,1), (35,35,1,7), (35,36,1,12), (35,37,1,6), (35,38,1,1), (35,43,1,1), (35,44,2,7), (35,46,1,1), (35,47,1,4), (35,48,1,5), (35,49,1,1), (35,50,2,7), (35,52,1,1), (35,57,1,1), (35,58,1,4), (35,59,1,11), (35,60,1,8), (35,61,2,1), (35,72,1,1), (35,73,1,11), (35,74,1,3), (35,75,3,1), (35,78,1,6), (35,79,1,4), (35,80,1,9), (35,81,1,1), (35,87,1,3), (35,88,1,8), (35,89,1,2), (35,90,1,11), (35,91,1,15), (35,92,1,4), (35,93,1,1), (36,2,1,1), (36,3,1,7), (36,4,1,15), (36,5,1,6), (36,6,1,2), (36,7,1,11), (36,8,1,1), (36,14,1,1), (36,15,1,10), (36,16,1,2), (36,17,1,6), (36,18,1,1), (36,21,1,1), (36,22,2,7), (36,24,1,1), (36,33,1,1), (36,34,1,9), (36,35,1,7), (36,36,1,9), (36,37,2,1), (36,43,1,1), (36,44,1,6), (36,45,1,5), (36,46,1,1), (36,47,1,4), (36,48,1,5), (36,49,1,1), (36,50,1,4), (36,51,1,7), (36,52,1,1), (36,57,2,1), (36,59,1,8), (36,60,1,7), (36,61,1,9), (36,62,1,2), (36,71,1,1), (36,72,1,7), (36,73,1,5), (36,74,1,1), (36,77,1,1), (36,78,1,6), (36,79,1,2), (36,80,1,10), (36,81,1,2), (36,82,1,1), (36,87,1,1), (36,88,1,10), (36,89,1,2), (36,90,1,5), (36,91,1,15), (36,92,1,8), (36,93,1,1), (37,2,1,1), (37,3,1,9), (37,4,1,14), (37,5,1,2), (37,6,1,4), (37,7,1,8), (37,8,3,1), (37,13,1,1), (37,14,1,4), (37,15,1,8), (37,16,2,4), (37,21,2,1), (37,23,1,11), (37,24,1,2), (37,25,1,1), (37,32,1,1), (37,33,1,7), (37,34,1,6), (37,35,1,10), (37,36,1,3), (37,37,4,1), (37,44,1,1), (37,45,1,8), (37,46,1,1), (37,47,1,3), (37,48,1,4), (37,49,2,1), (37,51,1,7), (37,52,1,1), (37,55,4,1), (37,59,1,2), (37,60,1,10), (37,61,1,4), (37,62,1,8), (37,63,1,1), (37,70,2,1), (37,72,1,9), (37,73,1,1), (37,78,1,3), (37,79,1,6), (37,80,1,7), (37,81,1,5), (37,82,6,1), (37,88,1,7), (37,89,1,5), (37,90,1,2), (37,91,1,13), (37,92,1,10), (37,93,1,1), (38,2,1,1), (38,3,1,12), (38,4,1,11), (38,5,1,2), (38,6,1,6), (38,7,1,5), (38,8,1,4), (38,9,1,3), (38,10,4,1), (38,14,1,7), (38,15,1,5), (38,16,1,8), (38,17,1,1), (38,22,1,1), (38,23,1,6), (38,24,1,7), (38,25,1,1), (38,31,1,1), (38,32,1,2), (38,33,1,8), (38,34,1,5), (38,35,1,10), (38,36,2,1), (38,38,3,5), (38,45,2,1), (38,47,1,3), (38,48,1,4), (38,49,1,1), (38,51,1,1), (38,55,3,5), (38,58,2,1), (38,60,1,8), (38,61,1,5), (38,62,1,8), (38,63,1,3), (38,64,1,1), (38,70,1,1), (38,71,1,6), (38,72,1,4), (38,73,1,1), (38,78,1,1), (38,79,1,8), (38,80,1,4), (38,81,1,8), (38,82,2,1), (38,84,1,7), (38,85,1,5), (38,86,1,1), (38,87,2,3), (38,89,1,7), (38,90,1,2), (38,91,1,9), (38,92,1,13), (38,93,1,2), (39,1,1,1), (39,2,1,2), (39,3,1,14), (39,4,1,7), (39,5,1,2), (39,6,1,8), (39,7,1,2), (39,8,1,7), (39,9,1,13), (39,10,1,12), (39,11,1,9), (39,12,1,4), (39,13,1,2), (39,14,1,10), (39,15,1,3), (39,16,1,7), (39,17,1,1), (39,23,1,1), (39,24,1,10), (39,25,1,3), (39,26,1,1), (39,31,1,1), (39,32,1,8), (39,33,1,9), (39,34,1,13), (39,35,1,3), (39,36,3,1), (39,39,1,10), (39,40,2,1), (39,46,1,1), (39,47,1,3), (39,48,1,4), (39,49,1,1), (39,55,1,1), (39,56,1,10), (39,57,1,2), (39,58,2,1), (39,60,1,3), (39,61,1,12), (39,62,2,9), (39,64,1,1), (39,69,1,1), (39,70,1,2), (39,71,1,9), (39,72,1,1), (39,78,1,1), (39,79,1,6), (39,80,1,3), (39,81,1,10), (39,82,1,2), (39,83,1,9), (39,84,1,7), (39,85,1,5), (39,86,1,9), (39,87,1,5), (39,88,1,1), (39,89,1,8), (39,90,1,2), (39,91,1,5), (39,92,1,15), (39,93,1,3), (39,94,1,1), (40,1,1,1), (40,2,1,3), (40,3,1,15), (40,4,1,13), (40,5,1,3), (40,6,1,8), (40,7,1,2), (40,8,1,10), (40,9,1,13), (40,10,1,12), (40,11,1,10), (40,12,2,2), (40,14,1,10), (40,15,1,5), (40,16,1,4), (40,17,1,1), (40,23,1,1), (40,24,1,5), (40,25,1,10), (40,26,1,1), (40,30,1,1), (40,31,1,2), (40,32,1,8), (40,33,1,9), (40,34,1,11), (40,35,1,8), (40,36,3,1), (40,39,1,4), (40,40,1,7), (40,46,1,1), (40,47,1,3), (40,48,1,5), (40,49,1,1), (40,55,1,1), (40,56,1,3), (40,57,1,6), (40,58,2,1), (40,60,1,8), (40,61,1,10), (40,62,1,9), (40,63,1,8), (40,64,1,3), (40,65,1,1), (40,69,1,1), (40,70,1,10), (40,71,1,4), (40,72,1,1), (40,79,1,2), (40,80,1,5), (40,81,1,10), (40,82,1,3), (40,83,1,6), (40,84,1,10), (40,85,1,9), (40,86,1,2), (40,87,2,1), (40,89,1,8), (40,90,1,3), (40,91,1,12), (40,92,1,15), (40,93,1,5), (40,94,1,1), (41,1,1,1), (41,2,1,5), (41,3,1,14), (41,4,1,4), (41,5,1,2), (41,6,1,8), (41,7,2,1), (41,9,2,2), (41,11,1,4), (41,12,1,2), (41,13,1,4), (41,14,1,9), (41,15,1,6), (41,16,1,1), (41,24,1,1), (41,25,1,10), (41,26,1,5), (41,27,1,1), (41,30,1,1), (41,31,1,9), (41,32,1,5), (41,33,1,11), (41,34,1,2), (41,35,1,4), (41,36,1,7), (41,37,1,5), (41,38,3,1), (41,46,1,1), (41,47,1,4), (41,48,1,5), (41,49,2,1), (41,55,2,1), (41,57,1,2), (41,58,1,5), (41,59,1,7), (41,60,1,4), (41,61,1,2), (41,62,1,10), (41,63,1,4), (41,64,1,10), (41,65,1,1), (41,68,1,1), (41,69,1,5), (41,70,1,9), (41,71,1,1), (41,79,1,1), (41,80,1,6), (41,81,1,8), (41,82,1,5), (41,83,1,2), (41,84,1,4), (41,85,4,1), (41,89,1,8), (41,90,1,2), (41,91,1,4), (41,92,1,13), (41,93,1,6), (41,94,1,1), (42,1,1,1), (42,2,1,7), (42,3,1,12), (42,4,2,2), (42,6,1,7), (42,7,1,1), (42,11,2,1), (42,13,2,7), (42,15,1,6), (42,16,1,1), (42,24,1,1), (42,25,1,2), (42,26,1,10), (42,27,3,1), (42,30,1,2), (42,31,1,11), (42,32,1,9), (42,33,1,4), (42,34,3,1), (42,37,1,4), (42,38,1,7), (42,39,1,4), (42,40,2,1), (42,44,1,1), (42,45,1,3), (42,46,1,7), (42,47,2,9), (42,49,1,7), (42,50,1,4), (42,51,1,1), (42,54,2,1), (42,56,1,4), (42,57,1,7), (42,58,1,4), (42,59,3,1), (42,62,1,3), (42,63,2,9), (42,65,1,3), (42,66,3,1), (42,69,1,9), (42,70,2,1), (42,79,1,1), (42,80,2,6), (42,82,1,8), (42,83,2,1), (42,88,1,1), (42,89,1,6), (42,90,1,3), (42,91,1,2), (42,92,1,11), (42,93,1,8), (42,94,1,1), (43,1,1,1), (43,2,1,8), (43,3,1,11), (43,4,1,2), (43,5,1,3), (43,6,1,6), (43,7,1,1), (43,12,1,1), (43,13,1,9), (43,14,1,5), (43,15,1,6), (43,16,1,1), (43,25,1,1), (43,26,2,6), (43,28,2,1), (43,30,1,6), (43,31,1,7), (43,32,1,11), (43,33,2,1), (43,35,1,3), (43,36,1,2), (43,37,1,4), (43,38,1,1), (43,39,1,6), (43,40,1,8), (43,41,1,2), (43,42,2,1), (43,44,1,5), (43,45,1,9), (43,46,1,2), (43,47,2,1), (43,49,1,2), (43,50,1,8), (43,51,1,6), (43,52,2,1), (43,54,1,2), (43,55,1,8), (43,56,1,6), (43,57,1,1), (43,58,1,4), (43,59,1,2), (43,60,1,5), (43,61,2,1), (43,63,1,9), (43,64,1,6), (43,65,1,7), (43,66,2,1), (43,68,2,5), (43,70,1,1), (43,79,1,1), (43,80,1,6), (43,81,1,4), (43,82,1,11), (43,83,1,1), (43,88,1,1), (43,89,1,5), (43,90,1,4), (43,91,1,2), (43,92,1,10), (43,93,1,9), (43,94,1,1), (44,1,1,1), (44,2,1,9), (44,3,1,10), (44,4,1,2), (44,5,1,4), (44,6,1,5), (44,7,1,1), (44,12,1,1), (44,13,1,11), (44,14,1,4), (44,15,1,6), (44,16,1,1), (44,25,1,1), (44,26,1,2), (44,27,1,10), (44,28,2,2), (44,30,1,8), (44,31,1,12), (44,32,1,5), (44,33,2,1), (44,35,1,5), (44,36,1,10), (44,37,1,3), (44,38,2,1), (44,40,1,2), (44,41,1,7), (44,42,2,5), (44,44,1,7), (44,45,6,1), (44,51,3,6), (44,54,1,7), (44,55,1,2), (44,56,2,1), (44,58,1,3), (44,59,1,10), (44,60,1,3), (44,61,2,1), (44,63,1,4), (44,64,1,11), (44,65,1,9), (44,66,2,2), (44,68,1,9), (44,69,1,1), (44,79,1,1), (44,80,1,6), (44,81,1,3), (44,82,1,12), (44,83,1,1), (44,88,1,1), (44,89,1,4), (44,90,1,5), (44,91,1,2), (44,92,1,9), (44,93,1,10), (44,94,1,1), (45,1,1,1), (45,2,1,10), (45,3,1,13), (45,4,2,5), (45,6,1,4), (45,7,1,1), (45,11,2,1), (45,13,1,12), (45,14,1,3), (45,15,1,6), (45,16,1,1), (45,26,1,1), (45,27,1,6), (45,28,1,10), (45,29,1,2), (45,30,1,11), (45,31,1,8), (45,32,1,1), (45,35,1,1), (45,36,1,7), (45,37,1,3), (45,38,1,1), (45,40,2,1), (45,42,1,5), (45,43,1,9), (45,44,1,1), (45,51,1,1), (45,52,1,8), (45,53,1,6), (45,54,2,1), (45,58,1,1), (45,59,1,4), (45,60,1,5), (45,61,1,1), (45,63,1,1), (45,64,1,6), (45,65,1,12), (45,66,1,2), (45,67,1,9), (45,68,1,5), (45,69,1,1), (45,79,1,1), (45,80,1,5), (45,81,1,2), (45,82,1,14), (45,83,1,1), (45,87,2,1), (45,89,1,3), (45,90,1,6), (45,91,1,5), (45,92,1,12), (45,93,1,11), (45,94,1,2), (46,1,1,1), (46,2,1,11), (46,3,1,9), (46,4,1,2), (46,5,1,6), (46,6,1,3), (46,7,1,1), (46,10,1,1), (46,11,1,3), (46,12,1,2), (46,13,1,14), (46,14,1,3), (46,15,1,5), (46,16,1,1), (46,27,1,1), (46,28,1,12), (46,29,1,6), (46,30,1,12), (46,31,1,2), (46,32,1,1), (46,36,1,1), (46,37,1,3), (46,41,1,1), (46,42,1,7), (46,43,1,2), (46,44,1,1), (46,51,1,1), (46,52,1,2), (46,53,1,8), (46,54,1,1), (46,59,1,1), (46,60,1,2), (46,63,1,1), (46,64,1,2), (46,65,1,11), (46,66,1,5), (46,67,1,11), (46,68,1,1), (46,79,1,1), (46,80,1,5), (46,81,1,3), (46,82,1,13), (46,83,1,2), (46,84,1,1), (46,86,1,1), (46,87,1,2), (46,88,1,6), (46,89,1,3), (46,90,1,7), (46,91,1,2), (46,92,1,8), (46,93,1,12), (46,94,1,2), (47,1,1,1), (47,2,1,11), (47,3,1,9), (47,4,1,2), (47,5,1,6), (47,6,1,4), (47,7,2,1), (47,9,1,3), (47,10,1,5), (47,11,1,10), (47,12,1,2), (47,13,1,14), (47,14,1,3), (47,15,1,5), (47,16,1,1), (47,27,1,1), (47,28,1,4), (47,29,1,13), (47,30,1,11), (47,31,1,2), (47,32,1,1), (47,41,1,1), (47,42,1,8), (47,43,2,1), (47,52,1,1), (47,53,1,7), (47,54,1,1), (47,64,1,1), (47,65,1,10), (47,66,1,12), (47,67,1,3), (47,68,1,1), (47,80,1,4), (47,81,1,3), (47,82,1,13), (47,83,1,2), (47,84,2,1), (47,86,1,3), (47,87,1,12), (47,88,1,9), (47,89,1,3), (47,90,1,7), (47,91,1,2), (47,92,1,8), (47,93,1,12), (47,94,1,2), (47,95,1,1), (48,1,1,1), (48,2,1,11), (48,3,1,9), (48,4,1,2), (48,5,1,6), (48,6,1,5), (48,7,1,10), (48,8,1,1), (48,9,1,4), (48,10,1,6), (48,11,1,10), (48,12,1,2), (48,13,1,14), (48,14,1,3), (48,15,1,5), (48,16,1,1), (48,27,1,1), (48,28,1,3), (48,29,1,13), (48,30,1,11), (48,31,1,2), (48,32,1,1), (48,41,1,1), (48,42,1,8), (48,43,2,1), (48,52,1,1), (48,53,1,7), (48,54,1,1), (48,64,1,1), (48,65,1,10), (48,66,1,13), (48,67,1,3), (48,68,1,1), (48,80,1,4), (48,81,1,3), (48,82,1,13), (48,83,1,3), (48,84,2,2), (48,86,1,5), (48,87,1,10), (48,88,2,3), (48,90,1,7), (48,91,1,2), (48,92,1,8), (48,93,1,12), (48,94,1,2), (48,95,1,1), (49,1,1,1), (49,2,1,11), (49,3,1,9), (49,4,1,2), (49,5,1,6), (49,6,1,4), (49,7,1,5), (49,8,3,1), (49,11,1,4), (49,12,1,2), (49,13,1,14), (49,14,1,3), (49,15,1,5), (49,16,1,1), (49,27,1,1), (49,28,1,11), (49,29,1,7), (49,30,1,11), (49,31,1,2), (49,32,1,1), (49,35,2,1), (49,37,1,2), (49,41,1,1), (49,42,1,7), (49,43,1,2), (49,44,1,1), (49,51,2,1), (49,53,1,8), (49,54,1,1), (49,58,1,2), (49,59,1,1), (49,60,1,3), (49,63,1,1), (49,64,1,2), (49,65,1,10), (49,66,1,5), (49,67,1,10), (49,68,1,1), (49,80,1,4), (49,81,1,3), (49,82,1,13), (49,83,1,3), (49,84,1,6), (49,85,1,10), (49,86,2,11), (49,88,1,10), (49,89,1,3), (49,90,1,7), (49,91,1,2), (49,92,1,8), (49,93,1,12), (49,94,1,2), (49,95,1,1), (50,1,1,1), (50,2,1,10), (50,3,1,13), (50,4,2,5), (50,6,1,4), (50,7,1,1), (50,11,2,1), (50,13,1,13), (50,14,1,3), (50,15,1,6), (50,16,1,1), (50,26,1,1), (50,27,1,5), (50,28,1,10), (50,29,1,2), (50,30,1,11), (50,31,1,7), (50,32,1,1), (50,34,1,1), (50,35,1,6), (50,36,1,8), (50,37,1,3), (50,38,1,1), (50,41,1,1), (50,42,1,5), (50,43,1,9), (50,44,1,1), (50,51,1,1), (50,52,1,7), (50,53,1,6), (50,54,1,1), (50,58,1,4), (50,59,1,9), (50,60,1,4), (50,61,1,1), (50,63,1,1), (50,64,1,6), (50,65,1,12), (50,66,1,2), (50,67,1,9), (50,68,1,5), (50,69,1,1), (50,79,1,1), (50,80,1,5), (50,81,1,2), (50,82,1,14), (50,83,1,2), (50,84,5,1), (50,89,1,3), (50,90,1,6), (50,91,1,5), (50,92,1,12), (50,93,1,11), (50,94,1,2), (51,1,1,1), (51,2,1,9), (51,3,1,10), (51,4,1,2), (51,5,1,4), (51,6,1,5), (51,7,1,1), (51,12,1,1), (51,13,1,11), (51,14,1,4), (51,15,1,6), (51,16,1,1), (51,26,1,1), (51,27,1,10), (51,28,2,2), (51,30,1,9), (51,31,1,13), (51,32,1,4), (51,33,3,1), (51,36,1,8), (51,37,1,2), (51,38,2,1), (51,40,1,2), (51,41,4,6), (51,45,2,1), (51,49,2,1), (51,51,1,5), (51,52,1,7), (51,53,2,6), (51,55,1,2), (51,56,3,1), (51,59,1,6), (51,60,1,4), (51,61,2,1), (51,63,1,3), (51,64,1,11), (51,65,1,9), (51,66,2,2), (51,68,1,9), (51,69,1,1), (51,79,1,1), (51,80,1,6), (51,81,1,3), (51,82,1,13), (51,83,1,1), (51,88,1,1), (51,89,1,4), (51,90,1,5), (51,91,1,2), (51,92,1,9), (51,93,1,10), (51,94,1,1), (52,1,1,1), (52,2,1,8), (52,3,1,11), (52,4,1,2), (52,5,1,3), (52,6,1,6), (52,7,1,1), (52,12,1,1), (52,13,1,9), (52,14,1,5), (52,15,1,6), (52,16,1,1), (52,25,1,1), (52,26,1,6), (52,27,1,7), (52,28,2,1), (52,30,1,6), (52,31,1,8), (52,32,1,10), (52,33,3,1), (52,36,1,3), (52,37,1,5), (52,38,1,1), (52,39,1,5), (52,40,1,9), (52,41,1,2), (52,42,2,1), (52,44,1,6), (52,45,1,8), (52,46,1,2), (52,47,3,1), (52,50,2,7), (52,52,2,1), (52,54,1,2), (52,55,1,9), (52,56,1,5), (52,57,3,1), (52,60,1,4), (52,61,2,1), (52,63,1,9), (52,64,1,5), (52,65,1,7), (52,66,2,1), (52,68,1,6), (52,69,1,5), (52,70,1,1), (52,79,1,1), (52,80,1,6), (52,81,1,4), (52,82,1,11), (52,83,1,1), (52,88,1,1), (52,89,1,5), (52,90,1,4), (52,91,1,2), (52,92,1,10), (52,93,1,9), (52,94,1,1), (53,1,1,1), (53,2,1,7), (53,3,1,12), (53,4,2,2), (53,6,1,7), (53,7,1,1), (53,12,1,1), (53,13,2,7), (53,15,1,6), (53,16,1,1), (53,24,1,1), (53,25,1,2), (53,26,1,10), (53,27,1,2), (53,28,2,1), (53,30,1,3), (53,31,2,10), (53,33,1,4), (53,34,3,1), (53,37,1,3), (53,38,1,7), (53,39,1,5), (53,40,2,1), (53,44,1,1), (53,45,1,4), (53,46,2,8), (53,48,1,9), (53,49,1,8), (53,50,1,4), (53,51,2,1), (53,54,2,1), (53,56,1,5), (53,57,1,7), (53,58,1,3), (53,59,3,1), (53,62,1,3), (53,63,2,9), (53,65,1,4), (53,66,3,1), (53,69,1,9), (53,70,2,1), (53,79,1,1), (53,80,2,6), (53,82,1,9), (53,83,1,1), (53,88,1,1), (53,89,1,6), (53,90,1,3), (53,91,1,2), (53,92,1,11), (53,93,1,8), (53,94,1,1), (54,1,1,1), (54,2,1,5), (54,3,1,14), (54,4,1,4), (54,5,1,2), (54,6,1,8), (54,7,4,1), (54,11,1,3), (54,12,1,2), (54,13,1,4), (54,14,1,9), (54,15,1,6), (54,16,1,1), (54,24,1,1), (54,25,1,9), (54,26,1,6), (54,27,1,1), (54,30,1,1), (54,31,1,9), (54,32,1,5), (54,33,1,11), (54,34,1,2), (54,35,1,3), (54,36,1,7), (54,37,1,6), (54,38,1,2), (54,39,2,1), (54,45,2,1), (54,47,1,4), (54,48,1,5), (54,49,2,1), (54,55,2,1), (54,57,1,2), (54,58,1,6), (54,59,1,7), (54,60,1,3), (54,61,1,2), (54,62,1,9), (54,63,1,4), (54,64,1,10), (54,65,1,1), (54,68,1,1), (54,69,1,5), (54,70,1,8), (54,71,1,1), (54,79,1,1), (54,80,1,6), (54,81,1,8), (54,82,1,5), (54,83,1,1), (54,87,2,1), (54,89,1,8), (54,90,1,2), (54,91,1,4), (54,92,1,13), (54,93,1,6), (54,94,1,1), (55,1,1,1), (55,2,1,4), (55,3,1,15), (55,4,1,13), (55,5,1,3), (55,6,1,8), (55,7,1,2), (55,8,1,3), (55,9,1,1), (55,10,1,2), (55,11,1,10), (55,12,2,2), (55,14,1,10), (55,15,1,5), (55,16,1,3), (55,17,1,1), (55,23,1,1), (55,24,1,4), (55,25,1,11), (55,26,1,1), (55,30,1,1), (55,31,1,3), (55,32,1,8), (55,33,1,10), (55,34,2,9), (55,36,1,2), (55,37,1,1), (55,38,1,7), (55,39,1,6), (55,40,1,7), (55,41,1,1), (55,46,1,1), (55,47,1,3), (55,48,1,5), (55,49,1,1), (55,55,1,7), (55,56,2,6), (55,58,2,1), (55,60,1,9), (55,61,1,8), (55,62,1,9), (55,63,1,7), (55,64,1,4), (55,65,1,1), (55,69,1,1), (55,70,1,10), (55,71,1,4), (55,72,1,1), (55,79,1,2), (55,80,1,6), (55,81,1,10), (55,82,1,3), (55,83,2,1), (55,87,2,2), (55,89,1,8), (55,90,1,3), (55,91,1,12), (55,92,1,15), (55,93,1,5), (55,94,1,1), (56,1,1,1), (56,2,1,2), (56,3,1,14), (56,4,1,6), (56,5,1,2), (56,6,1,9), (56,7,1,3), (56,8,1,10), (56,9,1,8), (56,10,1,11), (56,11,1,10), (56,12,1,4), (56,13,1,2), (56,14,1,10), (56,15,1,3), (56,16,1,7), (56,17,1,1), (56,23,1,1), (56,24,1,10), (56,25,1,3), (56,26,1,1), (56,31,1,1), (56,32,1,8), (56,33,1,9), (56,34,1,13), (56,35,1,3), (56,36,2,1), (56,38,1,2), (56,39,1,10), (56,40,2,1), (56,46,1,1), (56,47,1,3), (56,48,1,4), (56,49,1,1), (56,55,1,1), (56,56,1,10), (56,57,1,2), (56,58,2,1), (56,60,1,2), (56,61,1,13), (56,62,2,9), (56,64,1,1), (56,69,1,1), (56,70,1,3), (56,71,1,9), (56,72,1,1), (56,78,1,1), (56,79,1,6), (56,80,1,4), (56,81,1,10), (56,82,1,2), (56,83,1,4), (56,84,1,5), (56,85,2,1), (56,87,1,8), (56,88,1,3), (56,89,1,8), (56,90,1,2), (56,91,1,5), (56,92,1,15), (56,93,1,3), (56,94,1,1), (57,2,1,1), (57,3,1,12), (57,4,1,10), (57,5,1,2), (57,6,1,7), (57,7,1,5), (57,8,1,11), (57,9,1,7), (57,10,1,6), (57,11,3,1), (57,14,1,8), (57,15,1,4), (57,16,1,8), (57,17,1,1), (57,22,1,1), (57,23,1,5), (57,24,1,7), (57,25,1,1), (57,31,1,1), (57,32,1,3), (57,33,1,8), (57,34,1,6), (57,35,1,9), (57,36,3,1), (57,39,1,3), (57,40,1,6), (57,44,3,1), (57,47,1,3), (57,48,1,4), (57,49,3,1), (57,55,1,1), (57,56,1,3), (57,57,1,5), (57,58,2,1), (57,60,1,8), (57,61,1,5), (57,62,1,7), (57,63,1,3), (57,64,1,1), (57,70,1,1), (57,71,1,6), (57,72,1,4), (57,73,1,1), (57,78,1,1), (57,79,1,8), (57,80,1,3), (57,81,1,9), (57,82,2,1), (57,84,1,6), (57,85,1,4), (57,86,1,1), (57,87,1,9), (57,88,1,3), (57,89,1,7), (57,90,1,2), (57,91,1,9), (57,92,1,13), (57,93,1,2), (58,2,1,1), (58,3,1,10), (58,4,1,14), (58,5,1,2), (58,6,1,4), (58,7,1,9), (58,8,2,7), (58,10,1,3), (58,11,1,1), (58,13,1,1), (58,14,1,4), (58,15,1,8), (58,16,1,5), (58,17,1,4), (58,21,2,1), (58,23,1,10), (58,24,1,2), (58,25,1,1), (58,32,1,1), (58,33,1,7), (58,34,1,5), (58,35,1,11), (58,36,1,3), (58,37,1,1), (58,43,1,1), (58,44,1,7), (58,45,1,8), (58,46,1,1), (58,47,1,4), (58,48,1,5), (58,49,1,1), (58,50,2,7), (58,52,1,2), (58,57,2,1), (58,59,1,2), (58,60,1,10), (58,61,1,4), (58,62,1,8), (58,63,1,1), (58,70,2,1), (58,72,1,9), (58,73,2,1), (58,78,1,3), (58,79,1,6), (58,80,1,7), (58,81,1,5), (58,82,5,1), (58,87,1,4), (58,88,1,7), (58,89,1,5), (58,90,1,2), (58,91,1,12), (58,92,1,11), (58,93,1,1), (59,2,1,1), (59,3,1,7), (59,4,1,15), (59,5,1,5), (59,6,1,2), (59,7,1,11), (59,8,1,2), (59,9,2,1), (59,14,1,2), (59,15,1,10), (59,16,1,2), (59,17,1,6), (59,18,1,1), (59,21,1,1), (59,22,1,6), (59,23,1,8), (59,24,1,1), (59,33,1,2), (59,34,1,9), (59,35,1,7), (59,36,1,9), (59,37,2,1), (59,43,1,1), (59,44,1,7), (59,45,1,6), (59,46,1,1), (59,47,1,4), (59,48,1,5), (59,49,1,1), (59,50,1,5), (59,51,1,8), (59,52,1,1), (59,57,2,1), (59,59,1,8), (59,60,1,6), (59,61,1,10), (59,62,1,2), (59,71,1,1), (59,72,1,7), (59,73,1,5), (59,74,1,1), (59,77,1,1), (59,78,1,6), (59,79,1,2), (59,80,1,10), (59,81,1,2), (59,82,1,1), (59,86,2,1), (59,88,1,10), (59,89,1,3), (59,90,1,4), (59,91,1,15), (59,92,1,8), (59,93,1,1), (60,2,1,1), (60,3,1,3), (60,4,1,15), (60,5,1,12), (60,6,1,2), (60,7,1,8), (60,8,1,4), (60,9,1,1), (60,14,1,1), (60,15,1,9), (60,16,1,4), (60,17,1,6), (60,18,3,1), (60,21,1,4), (60,22,1,12), (60,23,1,2), (60,33,2,1), (60,35,1,8), (60,36,1,12), (60,37,1,5), (60,38,1,1), (60,44,1,1), (60,45,1,7), (60,46,1,1), (60,47,2,4), (60,49,2,1), (60,51,1,7), (60,52,1,1), (60,57,1,1), (60,58,1,4), (60,59,1,11), (60,60,1,9), (60,61,2,1), (60,72,1,1), (60,73,1,11), (60,74,1,3), (60,75,3,1), (60,78,1,6), (60,79,1,3), (60,80,1,10), (60,81,1,1), (60,87,1,3), (60,88,1,9), (60,89,1,2), (60,90,1,11), (60,91,1,15), (60,92,1,4), (60,93,1,1), (61,3,1,2), (61,4,1,13), (61,5,1,11), (61,6,1,2), (61,7,1,3), (61,8,1,7), (61,9,1,1), (61,14,1,1), (61,15,1,5), (61,16,1,10), (61,17,1,4), (61,18,1,5), (61,19,2,2), (61,21,1,10), (61,22,1,4), (61,23,1,1), (61,34,1,1), (61,35,1,2), (61,36,1,11), (61,37,1,13), (61,38,1,2), (61,39,1,1), (61,44,3,1), (61,47,1,4), (61,48,1,5), (61,49,4,1), (61,56,1,1), (61,57,1,2), (61,58,1,12), (61,59,1,11), (61,60,1,2), (61,61,1,1), (61,72,1,1), (61,73,1,3), (61,74,1,9), (61,75,2,2), (61,77,1,4), (61,78,1,5), (61,79,1,9), (61,80,1,6), (61,81,1,1), (61,86,1,1), (61,87,1,6), (61,88,1,4), (61,89,1,2), (61,90,1,10), (61,91,1,14), (61,92,1,2), (62,3,1,1), (62,4,1,7), (62,5,1,14), (62,6,1,3), (62,7,1,2), (62,8,1,8), (62,9,4,1), (62,15,1,1), (62,16,1,12), (62,17,1,4), (62,18,1,8), (62,19,2,10), (62,21,1,13), (62,22,1,3), (62,23,13,1), (62,36,1,3), (62,37,1,10), (62,38,1,9), (62,39,1,2), (62,40,7,1), (62,47,1,4), (62,48,1,5), (62,49,7,1), (62,56,1,2), (62,57,1,8), (62,58,1,10), (62,59,1,3), (62,60,13,1), (62,73,1,2), (62,74,1,12), (62,75,2,10), (62,77,1,8), (62,78,1,4), (62,79,1,13), (62,80,1,3), (62,81,1,4), (62,82,2,1), (62,86,1,1), (62,87,1,8), (62,88,2,2), (62,90,1,13), (62,91,1,9), (62,92,1,1), (63,3,1,1), (63,4,1,2), (63,5,1,14), (63,6,1,6), (63,7,1,2), (63,8,1,6), (63,9,1,3), (63,10,1,2), (63,11,1,5), (63,12,1,7), (63,13,3,1), (63,16,1,4), (63,17,2,10), (63,19,1,6), (63,20,2,11), (63,22,1,10), (63,23,1,7), (63,24,12,6), (63,36,1,7), (63,37,1,8), (63,38,1,14), (63,39,1,13), (63,40,1,8), (63,41,6,7), (63,47,2,9), (63,49,6,7), (63,55,1,8), (63,56,1,12), (63,57,1,14), (63,58,1,8), (63,59,1,7), (63,60,12,6), (63,72,1,7), (63,73,1,10), (63,74,2,11), (63,76,1,5), (63,77,1,11), (63,78,1,9), (63,79,1,5), (63,80,1,3), (63,81,1,11), (63,82,2,2), (63,84,2,1), (63,86,1,2), (63,87,1,7), (63,88,1,2), (63,89,1,5), (63,90,1,14), (63,91,1,3), (63,92,1,1), (64,4,1,1), (64,5,1,10), (64,6,1,12), (64,7,1,7), (64,8,1,3), (64,9,1,8), (64,10,1,4), (64,11,1,8), (64,12,1,11), (64,13,1,4), (64,14,3,1), (64,17,3,10), (64,20,1,5), (64,21,1,8), (64,22,1,9), (64,23,1,5), (64,24,12,4), (64,36,2,5), (64,38,1,8), (64,39,1,11), (64,40,1,6), (64,41,1,11), (64,42,1,12), (64,43,1,7), (64,44,3,5), (64,47,1,7), (64,48,1,8), (64,49,3,5), (64,52,1,6), (64,53,2,11), (64,55,1,6), (64,56,1,10), (64,57,1,8), (64,58,2,5), (64,60,12,4), (64,72,1,5), (64,73,1,9), (64,74,1,8), (64,75,1,5), (64,76,2,10), (64,78,1,11), (64,79,1,2), (64,80,1,8), (64,81,1,11), (64,82,1,13), (64,83,1,11), (64,84,1,12), (64,85,1,5), (64,86,1,7), (64,87,1,3), (64,88,1,7), (64,89,2,11), (64,91,1,2), (64,92,1,1), (65,4,1,1), (65,5,1,6), (65,6,1,15), (65,7,1,6), (65,8,1,2), (65,9,1,8), (65,10,1,2), (65,11,1,4), (65,12,1,14), (65,13,1,12), (65,14,1,2), (65,16,1,1), (65,17,1,4), (65,18,1,12), (65,19,1,11), (65,20,1,10), (65,21,1,9), (65,22,1,2), (65,23,15,1), (65,38,1,2), (65,39,1,11), (65,40,1,4), (65,41,1,2), (65,42,1,4), (65,43,1,7), (65,44,2,8), (65,46,1,9), (65,47,2,10), (65,49,1,9), (65,50,2,8), (65,52,1,7), (65,53,1,4), (65,54,1,2), (65,55,1,3), (65,56,1,10), (65,57,1,2), (65,58,15,1), (65,73,1,2), (65,74,1,9), (65,75,1,10), (65,76,2,11), (65,78,1,6), (65,79,1,2), (65,80,1,8), (65,81,1,6), (65,82,2,12), (65,84,1,9), (65,85,1,2), (65,86,1,8), (65,87,1,2), (65,88,1,5), (65,89,1,14), (65,90,1,7), (65,91,1,1), (66,4,1,1), (66,5,1,2), (66,6,1,13), (66,7,1,10), (66,8,1,2), (66,9,1,4), (66,10,1,8), (66,11,1,4), (66,12,1,7), (66,13,2,1), (66,17,1,1), (66,18,1,9), (66,19,2,6), (66,21,1,4), (66,22,1,1), (66,38,1,1), (66,39,1,4), (66,40,1,11), (66,41,1,2), (66,42,13,1), (66,55,1,10), (66,56,1,4), (66,57,1,1), (66,73,1,1), (66,74,1,3), (66,75,1,7), (66,76,1,5), (66,77,1,10), (66,78,4,1), (66,82,1,3), (66,83,2,2), (66,85,1,6), (66,86,1,5), (66,87,1,2), (66,88,1,9), (66,89,1,14), (66,90,1,2), (66,91,1,1), (67,5,1,1), (67,6,1,8), (67,7,1,15), (67,8,1,4), (67,9,1,2), (67,10,1,10), (67,11,1,3), (67,12,2,1), (67,17,1,1), (67,18,1,3), (67,19,1,13), (67,20,1,3), (67,21,1,7), (67,22,1,1), (67,39,1,1), (67,40,1,9), (67,41,1,6), (67,42,1,1), (67,53,1,1), (67,54,1,5), (67,55,1,9), (67,56,1,1), (67,73,1,1), (67,74,1,7), (67,75,1,3), (67,76,1,13), (67,77,1,3), (67,78,1,1), (67,82,2,1), (67,84,1,2), (67,85,1,11), (67,86,1,2), (67,87,1,3), (67,88,1,15), (67,89,1,9), (67,90,1,1), (68,5,1,1), (68,6,1,3), (68,7,1,14), (68,8,1,9), (68,9,2,3), (68,11,1,7), (68,12,1,1), (68,18,1,1), (68,19,1,5), (68,20,1,10), (68,21,1,4), (68,22,1,7), (68,23,1,1), (68,39,1,1), (68,40,1,2), (68,41,1,10), (68,42,1,1), (68,53,1,1), (68,54,1,9), (68,55,1,2), (68,56,1,1), (68,72,1,1), (68,73,1,6), (68,74,1,4), (68,75,1,9), (68,76,1,6), (68,77,1,2), (68,78,1,1), (68,83,1,1), (68,84,1,7), (68,85,1,4), (68,86,1,2), (68,87,1,8), (68,88,1,15), (68,89,1,4), (68,90,1,1), (69,6,1,1), (69,7,1,6), (69,8,1,15), (69,9,1,9), (69,10,1,2), (69,11,1,7), (69,12,1,3), (69,13,1,1), (69,17,1,1), (69,18,1,4), (69,19,1,2), (69,20,2,7), (69,22,1,3), (69,23,1,6), (69,24,1,1), (69,40,1,1), (69,41,1,6), (69,42,1,7), (69,43,1,1), (69,52,1,1), (69,53,1,5), (69,54,1,6), (69,55,1,1), (69,71,1,1), (69,72,1,6), (69,73,1,4), (69,74,1,6), (69,75,1,8), (69,76,1,3), (69,77,1,10), (69,78,1,1), (69,82,1,1), (69,83,1,2), (69,84,1,7), (69,85,1,2), (69,86,1,8), (69,87,1,14), (69,88,1,8), (69,89,1,1), (70,6,1,1), (70,7,1,2), (70,8,1,12), (70,9,1,10), (70,10,2,2), (70,12,1,8), (70,13,1,1), (70,16,1,1), (70,17,1,5), (70,18,1,7), (70,19,2,2), (70,21,1,10), (70,22,1,5), (70,23,1,4), (70,24,1,5), (70,25,1,1), (70,41,1,1), (70,42,1,11), (70,43,1,2), (70,44,1,1), (70,51,2,1), (70,53,1,10), (70,54,1,1), (70,70,1,1), (70,71,1,4), (70,72,1,5), (70,73,1,4), (70,74,1,11), (70,75,1,2), (70,76,1,5), (70,77,1,12), (70,78,1,5), (70,79,1,1), (70,82,1,1), (70,83,1,7), (70,84,2,2), (70,86,1,9), (70,87,1,13), (70,88,1,2), (70,89,1,1), (71,7,1,1), (71,8,1,6), (71,9,1,14), (71,10,1,5), (71,11,1,2), (71,12,1,6), (71,13,1,5), (71,14,2,1), (71,16,1,2), (71,17,1,12), (71,18,1,9), (71,19,1,8), (71,20,2,3), (71,22,1,11), (71,23,3,5), (71,26,1,1), (71,41,1,1), (71,42,1,6), (71,43,1,9), (71,44,1,1), (71,51,1,1), (71,52,1,7), (71,53,1,6), (71,54,1,1), (71,69,1,1), (71,70,1,4), (71,71,1,6), (71,72,1,4), (71,73,1,12), (71,74,1,3), (71,75,1,1), (71,76,1,3), (71,77,1,8), (71,78,1,9), (71,79,3,1), (71,82,1,4), (71,83,1,7), (71,84,1,2), (71,85,1,4), (71,86,1,14), (71,87,1,7), (71,88,1,1), (72,8,1,1), (72,9,1,11), (72,10,1,14), (72,11,1,4), (72,12,1,2), (72,13,1,8), (72,14,1,4), (72,15,1,2), (72,16,1,10), (72,17,1,5), (72,18,1,8), (72,19,1,2), (72,20,2,1), (72,22,1,2), (72,23,1,10), (72,24,1,5), (72,25,1,4), (72,26,1,6), (72,27,1,1), (72,42,1,1), (72,43,1,10), (72,44,1,5), (72,45,1,1), (72,50,1,1), (72,51,1,4), (72,52,1,10), (72,53,1,1), (72,68,1,1), (72,69,1,6), (72,70,1,5), (72,71,1,4), (72,72,1,10), (72,73,1,2), (72,74,1,1), (72,76,1,1), (72,77,1,2), (72,78,1,13), (72,79,1,6), (72,80,1,1), (72,81,1,3), (72,82,1,9), (72,83,1,2), (72,84,1,3), (72,85,1,13), (72,86,1,12), (72,87,1,2), (72,88,1,1), (73,8,1,1), (73,9,1,4), (73,10,1,15), (73,11,1,13), (73,12,1,6), (73,13,1,2), (73,14,1,9), (73,15,1,2), (73,16,1,5), (73,17,1,6), (73,18,1,11), (73,19,1,2), (73,20,1,1), (73,22,1,1), (73,23,1,2), (73,24,1,11), (73,25,1,5), (73,26,1,3), (73,27,1,6), (73,28,1,1), (73,42,1,1), (73,43,1,2), (73,44,1,10), (73,45,1,1), (73,50,1,1), (73,51,1,9), (73,52,1,2), (73,53,1,1), (73,67,1,1), (73,68,1,6), (73,69,2,4), (73,71,1,12), (73,72,1,2), (73,73,1,1), (73,77,1,1), (73,78,1,9), (73,79,1,6), (73,80,1,2), (73,81,1,9), (73,82,1,2), (73,83,1,6), (73,84,1,12), (73,85,1,15), (73,86,1,5), (73,87,1,1), (74,9,1,1), (74,10,1,7), (74,11,1,15), (74,12,1,5), (74,13,1,2), (74,14,1,4), (74,15,1,7), (74,16,1,2), (74,17,1,6), (74,18,2,1), (74,23,1,1), (74,24,1,3), (74,25,1,10), (74,26,1,7), (74,27,1,4), (74,28,1,7), (74,29,1,3), (74,30,1,1), (74,43,1,1), (74,44,1,8), (74,45,1,5), (74,46,1,1), (74,49,1,1), (74,50,1,3), (74,51,1,7), (74,52,1,1), (74,65,1,1), (74,66,1,2), (74,67,1,7), (74,68,1,4), (74,69,1,6), (74,70,1,11), (74,71,1,4), (74,72,1,1), (74,77,1,1), (74,78,1,4), (74,79,1,3), (74,80,1,6), (74,81,1,4), (74,82,1,2), (74,83,1,5), (74,84,1,14), (74,85,1,8), (74,86,1,1), (75,9,2,1), (75,11,1,8), (75,12,1,13), (75,13,1,4), (75,14,1,2), (75,15,1,5), (75,16,1,7), (75,17,2,1), (75,22,1,1), (75,23,1,2), (75,24,1,3), (75,25,1,1), (75,26,1,7), (75,27,1,9), (75,28,1,3), (75,29,2,5), (75,31,2,1), (75,43,1,1), (75,44,1,2), (75,45,1,10), (75,46,1,2), (75,47,3,1), (75,50,1,9), (75,51,1,2), (75,63,2,1), (75,65,2,5), (75,67,1,2), (75,68,1,9), (75,69,1,8), (75,70,1,2), (75,71,2,3), (75,73,1,1), (75,78,1,1), (75,79,2,6), (75,81,1,2), (75,82,1,3), (75,83,1,13), (75,84,1,9), (75,85,1,2), (75,86,1,1), (76,10,1,1), (76,11,1,2), (76,12,2,13), (76,14,1,3), (76,15,1,2), (76,16,2,6), (76,18,1,1), (76,21,1,1), (76,22,1,2), (76,23,1,7), (76,24,1,4), (76,25,2,1), (76,27,1,5), (76,28,1,13), (76,29,1,5), (76,30,1,3), (76,31,1,8), (76,32,1,4), (76,33,2,1), (76,44,1,1), (76,45,1,7), (76,46,1,8), (76,47,1,2), (76,48,1,1), (76,49,1,5), (76,50,1,7), (76,51,1,1), (76,62,1,1), (76,63,1,3), (76,64,1,8), (76,65,1,3), (76,66,1,4), (76,67,1,13), (76,68,1,6), (76,69,1,1), (76,70,1,3), (76,71,1,9), (76,72,1,10), (76,73,1,1), (76,77,1,1), (76,78,1,5), (76,79,1,7), (76,80,1,2), (76,81,1,3), (76,82,1,13), (76,83,1,14), (76,84,1,2), (76,85,1,1), (77,11,1,1), (77,12,1,5), (77,13,1,14), (77,14,1,12), (77,15,1,6), (77,16,1,2), (77,17,1,6), (77,18,1,4), (77,19,3,1), (77,22,1,9), (77,23,1,13), (77,24,1,2), (77,25,3,1), (77,28,1,3), (77,29,1,9), (77,30,1,8), (77,31,1,3), (77,32,1,5), (77,33,1,6), (77,34,1,4), (77,35,2,1), (77,44,1,1), (77,45,1,2), (77,46,1,12), (77,47,1,5), (77,48,1,4), (77,49,1,12), (77,50,1,2), (77,51,1,1), (77,59,2,1), (77,61,1,3), (77,62,1,6), (77,63,1,5), (77,64,1,2), (77,65,1,8), (77,66,1,10), (77,67,1,3), (77,68,3,1), (77,71,1,5), (77,72,1,12), (77,73,1,2), (77,74,1,1), (77,76,1,1), (77,77,1,3), (77,78,1,7), (77,79,1,2), (77,80,1,6), (77,81,1,11), (77,82,1,14), (77,83,1,5), (77,84,1,1), (78,12,1,1), (78,13,1,4), (78,14,1,13), (78,15,1,10), (78,16,2,2), (78,18,2,6), (78,20,1,2), (78,21,1,5), (78,22,1,11), (78,23,1,15), (78,24,1,7), (78,25,1,1), (78,28,2,1), (78,30,1,4), (78,31,1,10), (78,32,1,9), (78,33,1,3), (78,34,1,4), (78,35,2,6), (78,37,1,3), (78,38,3,1), (78,44,1,1), (78,45,1,2), (78,46,1,10), (78,47,1,13), (78,48,1,12), (78,49,1,10), (78,50,1,2), (78,51,1,1), (78,55,3,1), (78,58,1,3), (78,59,2,6), (78,61,1,4), (78,62,1,3), (78,63,1,9), (78,64,1,10), (78,65,1,5), (78,66,2,1), (78,69,1,1), (78,70,1,5), (78,71,1,7), (78,72,1,4), (78,73,3,1), (78,76,1,5), (78,77,1,7), (78,78,2,2), (78,80,1,9), (78,81,1,14), (78,82,1,5), (78,83,1,1), (79,13,1,1), (79,14,1,5), (79,15,1,15), (79,16,1,9), (79,17,2,2), (79,19,1,6), (79,20,1,7), (79,21,1,6), (79,22,1,15), (79,23,1,8), (79,24,1,4), (79,25,1,1), (79,30,2,1), (79,32,1,4), (79,33,1,12), (79,34,1,9), (79,35,1,4), (79,36,1,2), (79,37,1,6), (79,38,1,8), (79,39,1,6), (79,40,1,2), (79,41,4,1), (79,45,1,3), (79,46,3,10), (79,49,1,8), (79,50,1,4), (79,51,4,1), (79,55,1,2), (79,56,1,6), (79,57,1,8), (79,58,1,6), (79,59,1,2), (79,60,1,3), (79,61,1,9), (79,62,1,12), (79,63,1,5), (79,64,1,2), (79,65,2,1), (79,70,1,4), (79,71,1,9), (79,72,1,11), (79,73,1,4), (79,74,1,1), (79,75,1,6), (79,76,1,7), (79,77,2,2), (79,79,1,8), (79,80,1,15), (79,81,1,6), (79,82,2,1), (80,13,2,1), (80,15,1,7), (80,16,1,15), (80,17,1,10), (80,18,1,6), (80,19,1,2), (80,20,1,6), (80,21,1,7), (80,22,1,5), (80,23,1,8), (80,24,1,1), (80,30,2,1), (80,32,2,2), (80,34,1,5), (80,35,1,9), (80,36,1,10), (80,37,1,7), (80,38,1,4), (80,39,1,3), (80,40,1,5), (80,41,4,6), (80,45,1,7), (80,46,1,11), (80,47,2,7), (80,49,1,10), (80,50,1,8), (80,51,4,6), (80,55,1,5), (80,56,1,4), (80,57,1,3), (80,58,1,7), (80,59,2,10), (80,61,1,6), (80,62,3,2), (80,65,1,5), (80,66,1,1), (80,70,2,1), (80,72,2,2), (80,74,2,6), (80,76,1,2), (80,77,1,6), (80,78,1,9), (80,79,1,15), (80,80,1,8), (80,81,2,1), (81,15,1,1), (81,16,1,5), (81,17,1,13), (81,18,1,12), (81,19,1,3), (81,20,1,2), (81,21,1,4), (81,22,1,9), (81,23,1,4), (81,24,2,1), (81,29,2,1), (81,31,1,4), (81,32,1,7), (81,33,3,1), (81,36,1,2), (81,37,1,5), (81,38,1,9), (81,39,2,10), (81,41,1,8), (81,42,1,6), (81,43,1,4), (81,44,2,3), (81,46,1,6), (81,47,2,11), (81,49,1,7), (81,50,2,3), (81,52,1,4), (81,53,1,6), (81,54,1,8), (81,55,2,10), (81,57,1,9), (81,58,1,6), (81,59,1,2), (81,60,2,1), (81,62,1,3), (81,63,1,8), (81,64,1,7), (81,65,1,11), (81,66,1,2), (81,70,2,1), (81,72,1,3), (81,73,1,9), (81,74,1,4), (81,75,1,2), (81,76,1,3), (81,77,1,11), (81,78,1,14), (81,79,1,6), (81,80,1,1), (82,16,1,1), (82,17,1,4), (82,18,1,14), (82,19,1,13), (82,20,1,4), (82,21,2,2), (82,23,1,9), (82,24,1,5), (82,25,2,1), (82,28,1,1), (82,29,1,4), (82,30,1,9), (82,31,1,13), (82,32,1,6), (82,33,1,1), (82,36,2,1), (82,38,1,3), (82,39,1,2), (82,40,1,3), (82,41,1,5), (82,42,1,9), (82,43,1,11), (82,44,1,12), (82,45,1,14), (82,46,4,13), (82,50,1,14), (82,51,1,13), (82,52,1,11), (82,53,1,9), (82,54,1,6), (82,55,1,3), (82,56,2,2), (82,58,2,1), (82,62,1,1), (82,63,1,2), (82,64,1,4), (82,65,1,7), (82,66,1,1), (82,69,2,1), (82,71,1,4), (82,72,1,9), (82,73,2,2), (82,75,1,3), (82,76,1,13), (82,77,1,14), (82,78,1,5), (82,79,2,1), (83,17,1,1), (83,18,1,5), (83,19,2,13), (83,21,1,5), (83,22,1,6), (83,23,1,2), (83,24,1,7), (83,25,1,8), (83,26,1,3), (83,27,2,1), (83,29,1,3), (83,30,1,9), (83,31,1,12), (83,32,1,4), (83,33,1,1), (83,37,1,1), (83,38,1,11), (83,39,1,9), (83,40,1,7), (83,41,1,3), (83,42,3,1), (83,45,1,2), (83,46,4,3), (83,50,1,2), (83,51,3,1), (83,54,1,3), (83,55,1,7), (83,56,1,9), (83,57,1,7), (83,58,1,1), (83,63,1,1), (83,64,1,10), (83,65,1,3), (83,66,3,1), (83,69,1,2), (83,70,1,8), (83,71,1,7), (83,72,1,2), (83,73,1,6), (83,74,1,5), (83,75,1,13), (83,76,1,14), (83,77,1,5), (83,78,2,1), (84,18,1,1), (84,19,1,2), (84,20,1,8), (84,21,1,15), (84,22,1,13), (84,23,1,4), (84,24,2,2), (84,26,2,7), (84,28,2,2), (84,30,1,7), (84,31,1,11), (84,32,1,2), (84,37,2,1), (84,39,1,2), (84,40,1,7), (84,41,1,6), (84,42,1,1), (84,45,1,1), (84,46,1,3), (84,47,1,10), (84,48,1,9), (84,49,1,4), (84,50,1,1), (84,54,1,1), (84,55,1,3), (84,56,1,2), (84,57,1,9), (84,58,1,1), (84,63,1,1), (84,64,1,12), (84,65,2,1), (84,67,1,2), (84,68,2,7), (84,70,2,2), (84,72,1,3), (84,73,1,12), (84,74,1,14), (84,75,1,9), (84,76,1,2), (84,77,1,1), (85,19,1,1), (85,20,1,2), (85,21,1,7), (85,22,1,15), (85,23,1,13), (85,24,1,4), (85,25,2,2), (85,27,1,4), (85,28,1,10), (85,29,1,7), (85,30,1,2), (85,31,1,4), (85,32,2,1), (85,37,1,1), (85,38,1,8), (85,39,1,2), (85,40,1,9), (85,41,1,4), (85,42,1,1), (85,45,1,1), (85,46,1,2), (85,47,1,11), (85,48,1,2), (85,49,1,1), (85,54,1,1), (85,55,1,2), (85,56,1,3), (85,57,1,8), (85,58,1,1), (85,63,1,1), (85,64,1,4), (85,65,1,1), (85,66,1,6), (85,67,1,11), (85,68,1,4), (85,69,2,2), (85,71,1,4), (85,72,1,13), (85,73,1,15), (85,74,1,8), (85,75,1,2), (85,76,1,1), (86,20,2,1), (86,22,1,4), (86,23,1,11), (86,24,1,14), (86,25,2,9), (86,27,1,3), (86,28,1,2), (86,29,1,5), (86,30,1,8), (86,31,1,7), (86,32,1,3), (86,33,2,1), (86,37,1,1), (86,38,2,6), (86,40,1,12), (86,41,1,2), (86,45,1,1), (86,46,1,5), (86,47,1,9), (86,48,1,2), (86,49,1,1), (86,54,1,1), (86,55,1,12), (86,56,1,4), (86,57,1,7), (86,58,1,1), (86,61,2,1), (86,63,1,2), (86,64,1,7), (86,65,1,8), (86,66,1,5), (86,67,2,2), (86,69,1,8), (86,70,1,9), (86,71,1,14), (86,72,1,12), (86,73,1,5), (86,74,2,1), (87,22,2,1), (87,24,1,6), (87,25,1,12), (87,26,1,14), (87,27,1,9), (87,28,1,4), (87,29,2,2), (87,31,1,3), (87,32,1,7), (87,33,1,8), (87,34,1,6), (87,35,1,3), (87,36,2,1), (87,38,1,2), (87,39,1,8), (87,40,1,11), (87,41,1,1), (87,45,1,1), (87,46,1,2), (87,47,1,10), (87,48,1,2), (87,54,1,1), (87,55,1,11), (87,56,1,10), (87,57,1,5), (87,58,2,1), (87,60,1,3), (87,61,1,6), (87,62,1,8), (87,63,1,7), (87,64,1,3), (87,65,2,2), (87,67,1,3), (87,68,1,8), (87,69,1,14), (87,70,1,13), (87,71,1,7), (87,72,1,2), (87,73,1,1), (88,24,1,1), (88,25,1,2), (88,26,1,7), (88,27,2,15), (88,29,1,10), (88,30,1,6), (88,31,1,7), (88,32,2,2), (88,34,1,3), (88,35,1,8), (88,36,1,10), (88,37,1,7), (88,38,1,3), (88,39,2,2), (88,41,5,1), (88,46,1,12), (88,47,1,7), (88,48,7,1), (88,55,1,2), (88,56,2,3), (88,58,1,7), (88,59,1,10), (88,60,1,9), (88,61,1,4), (88,62,2,2), (88,64,1,7), (88,65,1,5), (88,66,1,9), (88,67,2,15), (88,69,1,8), (88,70,1,2), (88,71,2,1), (89,25,2,1), (89,27,1,4), (89,28,1,9), (89,29,1,13), (89,30,1,14), (89,31,1,11), (89,32,1,6), (89,33,4,2), (89,37,1,5), (89,38,1,7), (89,39,3,8), (89,42,1,6), (89,43,1,5), (89,44,2,4), (89,46,1,7), (89,47,1,4), (89,48,2,2), (89,50,1,3), (89,51,1,4), (89,52,1,5), (89,53,1,6), (89,54,3,8), (89,57,1,7), (89,58,1,5), (89,59,1,3), (89,60,3,2), (89,63,1,5), (89,64,1,11), (89,65,2,14), (89,67,1,9), (89,68,1,4), (89,69,2,1), (90,27,2,1), (90,29,1,2), (90,30,1,6), (90,31,1,11), (90,32,2,14), (90,34,1,10), (90,35,1,11), (90,36,1,5), (90,37,5,2), (90,42,1,3), (90,43,1,4), (90,44,1,5), (90,45,1,6), (90,46,4,7), (90,50,1,6), (90,51,1,5), (90,52,1,4), (90,53,1,3), (90,54,5,2), (90,59,1,4), (90,60,1,11), (90,61,1,10), (90,62,1,13), (90,63,1,14), (90,64,1,11), (90,65,1,7), (90,66,1,2), (90,67,2,1), (91,29,2,1), (91,31,1,2), (91,32,1,3), (91,33,1,8), (91,34,1,13), (91,35,2,15), (91,37,1,13), (91,38,1,9), (91,39,1,5), (91,40,1,12), (91,41,1,4), (91,42,3,2), (91,45,1,5), (91,46,4,2), (91,50,1,5), (91,51,3,2), (91,54,1,4), (91,55,1,12), (91,56,1,5), (91,57,1,9), (91,58,1,12), (91,59,2,15), (91,61,1,14), (91,62,1,9), (91,63,1,3), (91,64,1,2), (91,65,2,1), (92,31,3,1), (92,34,1,2), (92,35,1,4), (92,36,1,8), (92,37,1,10), (92,38,1,13), (92,39,2,15), (92,41,1,13), (92,42,1,11), (92,43,1,10), (92,44,1,9), (92,45,1,12), (92,46,4,8), (92,50,1,12), (92,51,1,9), (92,52,1,10), (92,53,1,11), (92,54,1,13), (92,55,2,15), (92,57,1,13), (92,58,1,11), (92,59,1,8), (92,60,1,4), (92,61,1,2), (92,62,3,1), (93,35,3,1), (93,38,1,2), (93,39,1,3), (93,40,1,5), (93,41,1,6), (93,42,1,8), (93,43,1,9), (93,44,1,10), (93,45,1,11), (93,46,4,12), (93,50,1,11), (93,51,1,10), (93,52,1,9), (93,53,1,8), (93,54,1,6), (93,55,1,5), (93,56,1,3), (93,57,1,2), (93,58,3,1), (94,39,6,1), (94,45,6,2), (94,51,6,1), (95,47,3,1)];
pub const PIXEL_CIRCLE_SIZE: usize = 64;
pub static PIXEL_CIRCLE: [(u8, u8, u8, u8); 664] = [(1,27,10,15), (2,23,4,15), (2,28,1,15), (2,35,1,15), (2,37,4,15), (3,20,3,15), (3,27,10,15), (3,41,3,15), (4,18,2,15), (4,22,5,15), (4,37,5,15), (4,44,2,15), (5,16,2,15), (5,20,3,15), (5,27,1,15), (5,30,1,15), (5,41,3,15), (5,46,2,15), (6,15,1,15), (6,18,2,15), (6,25,2,15), (6,30,1,15), (6,32,1,15), (6,37,1,15), (6,44,2,15), (6,48,1,15), (7,13,2,15), (7,16,2,15), (7,25,1,15), (7,30,1,15), (7,32,1,15), (7,36,2,15), (7,46,2,15), (7,49,2,15), (8,12,1,15), (8,15,2,15), (8,20,2,15), (8,25,1,15), (8,27,12,15), (8,42,1,15), (8,44,1,15), (8,47,2,15), (8,51,1,15), (9,11,1,15), (9,14,1,15), (9,21,2,15), (9,24,3,15), (9,31,3,15), (9,37,3,15), (9,41,1,15), (9,43,1,15), (9,49,1,15), (9,52,1,15), (10,10,1,15), (10,13,1,15), (10,16,1,15), (10,20,4,15), (10,27,5,15), (10,33,4,15), (10,40,3,15), (10,50,1,15), (10,53,1,15), (11,9,1,15), (11,11,2,15), (11,14,1,15), (11,16,1,15), (11,20,1,15), (11,24,3,15), (11,31,3,15), (11,37,3,15), (11,43,1,15), (11,49,1,15), (11,51,2,15), (11,54,1,15), (12,8,1,15), (12,11,1,15), (12,15,2,15), (12,18,2,15), (12,22,2,15), (12,31,1,15), (12,33,1,15), (12,40,2,15), (12,44,4,15), (12,52,1,15), (12,55,1,15), (13,7,1,15), (13,10,1,15), (13,17,1,15), (13,20,2,15), (13,30,1,15), (13,34,1,15), (13,42,2,15), (13,46,1,15), (13,48,1,15), (13,53,1,15), (13,56,1,15), (14,7,1,15), (14,9,1,15), (14,11,1,15), (14,16,1,15), (14,19,1,15), (14,30,1,15), (14,34,1,15), (14,44,1,15), (14,47,1,15), (14,52,1,15), (14,54,1,15), (14,56,1,15), (15,6,1,15), (15,8,1,15), (15,13,1,15), (15,15,1,15), (15,18,1,15), (15,29,1,15), (15,35,1,15), (15,45,1,15), (15,48,1,15), (15,51,1,15), (15,55,1,15), (15,57,1,15), (16,5,1,15), (16,7,2,15), (16,12,1,15), (16,14,1,15), (16,17,1,15), (16,28,1,15), (16,35,1,15), (16,46,1,15), (16,49,1,15), (16,51,3,15), (16,55,2,15), (16,58,1,15), (17,5,1,15), (17,7,1,15), (17,12,2,15), (17,16,1,15), (17,28,1,15), (17,36,1,15), (17,47,1,15), (17,50,1,15), (17,56,1,15), (17,58,1,15), (18,4,1,15), (18,6,1,15), (18,12,1,15), (18,15,1,15), (18,27,1,15), (18,37,1,15), (18,48,1,15), (18,51,1,15), (18,57,1,15), (18,59,1,15), (19,4,1,15), (19,6,1,15), (19,8,1,15), (19,12,1,15), (19,14,1,15), (19,27,1,15), (19,37,1,15), (19,49,1,15), (19,51,1,15), (19,57,1,15), (19,59,1,15), (20,3,1,15), (20,5,1,15), (20,9,1,15), (20,11,4,15), (20,26,1,15), (20,29,6,15), (20,38,1,15), (20,49,5,15), (20,55,1,15), (20,58,1,15), (20,60,1,15), (21,3,1,15), (21,5,1,15), (21,8,1,15), (21,10,1,15), (21,12,1,15), (21,14,36,15), (21,51,1,15), (21,53,3,15), (21,58,1,15), (21,60,1,15), (22,3,3,15), (22,9,2,15), (22,12,3,15), (22,25,1,15), (22,32,1,15), (22,38,2,15), (22,49,3,15), (22,53,2,15), (22,58,3,15), (23,2,1,15), (23,4,1,15), (23,10,1,15), (23,12,1,15), (23,14,1,15), (23,24,1,15), (23,28,1,15), (23,30,1,15), (23,32,2,15), (23,35,1,15), (23,39,1,15), (23,49,1,15), (23,51,1,15), (23,53,1,15), (23,59,1,15), (23,61,1,15), (24,2,1,15), (24,4,1,15), (24,9,1,15), (24,11,1,15), (24,15,1,15), (24,23,2,15), (24,29,1,15), (24,32,1,15), (24,34,1,15), (24,40,1,15), (24,48,1,15), (24,52,1,15), (24,54,1,15), (24,59,1,15), (24,61,1,15), (25,2,1,15), (25,4,1,15), (25,8,2,15), (25,11,1,15), (25,15,1,15), (25,22,2,15), (25,25,1,15), (25,27,1,15), (25,30,1,15), (25,32,1,15), (25,35,2,15), (25,38,1,15), (25,40,2,15), (25,48,1,15), (25,52,1,15), (25,54,4,15), (25,59,1,15), (25,61,1,15), (26,2,1,15), (26,4,1,15), (26,6,4,15), (26,11,1,15), (26,16,1,15), (26,21,3,15), (26,26,1,15), (26,32,1,15), (26,37,1,15), (26,40,3,15), (26,47,1,15), (26,52,1,15), (26,54,1,15), (26,57,1,15), (26,59,1,15), (26,61,1,15), (27,1,1,15), (27,3,1,15), (27,7,2,15), (27,10,1,15), (27,17,1,15), (27,21,2,15), (27,24,2,15), (27,27,1,15), (27,32,1,15), (27,38,2,15), (27,42,1,15), (27,47,1,15), (27,53,1,15), (27,55,1,15), (27,58,1,15), (27,60,1,15), (27,62,1,15), (28,1,3,15), (28,8,1,15), (28,10,1,15), (28,17,1,15), (28,21,1,15), (28,23,1,15), (28,25,3,15), (28,30,4,15), (28,36,3,15), (28,40,1,15), (28,42,1,15), (28,46,1,15), (28,53,1,15), (28,55,1,15), (28,60,3,15), (29,1,1,15), (29,3,1,15), (29,8,1,15), (29,10,1,15), (29,18,1,15), (29,20,2,15), (29,24,1,15), (29,28,2,15), (29,34,2,15), (29,39,1,15), (29,43,1,15), (29,45,1,15), (29,53,1,15), (29,55,1,15), (29,60,1,15), (29,62,1,15), (30,1,1,15), (30,3,1,15), (30,8,1,15), (30,10,1,15), (30,19,2,15), (30,25,1,15), (30,28,1,15), (30,35,1,15), (30,40,1,15), (30,43,1,15), (30,45,1,15), (30,53,1,15), (30,55,4,15), (30,60,1,15), (30,62,1,15), (31,1,1,15), (31,3,1,15), (31,7,2,15), (31,10,1,15), (31,19,2,15), (31,28,1,15), (31,35,1,15), (31,43,2,15), (31,53,1,15), (31,55,1,15), (31,60,1,15), (31,62,1,15), (32,1,1,15), (32,3,1,15), (32,5,4,15), (32,10,1,15), (32,19,2,15), (32,28,1,15), (32,35,1,15), (32,43,2,15), (32,53,1,15), (32,55,3,15), (32,60,1,15), (32,62,1,15), (33,1,1,15), (33,3,1,15), (33,8,1,15), (33,10,1,15), (33,18,1,15), (33,20,1,15), (33,23,1,15), (33,25,1,15), (33,28,1,15), (33,35,1,15), (33,38,1,15), (33,40,1,15), (33,43,1,15), (33,45,1,15), (33,53,1,15), (33,55,1,15), (33,60,1,15), (33,62,1,15), (34,1,1,15), (34,3,1,15), (34,8,1,15), (34,10,1,15), (34,18,1,15), (34,20,2,15), (34,24,1,15), (34,28,2,15), (34,34,2,15), (34,39,1,15), (34,42,2,15), (34,46,1,15), (34,53,1,15), (34,55,1,15), (34,60,1,15), (34,62,1,15), (35,1,3,15), (35,8,1,15), (35,10,1,15), (35,17,1,15), (35,21,2,15), (35,25,3,15), (35,30,4,15), (35,36,2,15), (35,40,1,15), (35,42,1,15), (35,46,1,15), (35,53,1,15), (35,55,1,15), (35,60,3,15), (36,1,1,15), (36,3,1,15), (36,5,1,15), (36,8,1,15), (36,10,1,15), (36,17,1,15), (36,21,2,15), (36,24,2,15), (36,27,1,15), (36,32,1,15), (36,36,1,15), (36,38,2,15), (36,41,2,15), (36,47,1,15), (36,53,1,15), (36,55,2,15), (36,60,1,15), (36,62,1,15), (37,2,1,15), (37,4,1,15), (37,6,1,15), (37,9,1,15), (37,11,1,15), (37,16,1,15), (37,21,3,15), (37,26,1,15), (37,32,1,15), (37,37,1,15), (37,40,3,15), (37,47,1,15), (37,52,1,15), (37,54,4,15), (37,59,1,15), (37,61,1,15), (38,2,1,15), (38,4,1,15), (38,6,4,15), (38,11,1,15), (38,15,1,15), (38,22,2,15), (38,27,2,15), (38,30,1,15), (38,32,2,15), (38,35,1,15), (38,38,1,15), (38,40,2,15), (38,48,1,15), (38,52,1,15), (38,54,2,15), (38,59,1,15), (38,61,1,15), (39,2,1,15), (39,4,1,15), (39,9,1,15), (39,11,1,15), (39,15,1,15), (39,23,2,15), (39,29,1,15), (39,32,1,15), (39,34,1,15), (39,40,1,15), (39,48,1,15), (39,52,1,15), (39,54,1,15), (39,59,1,15), (39,61,1,15), (40,2,1,15), (40,4,1,15), (40,10,1,15), (40,12,1,15), (40,14,1,15), (40,24,2,15), (40,30,1,15), (40,32,1,15), (40,35,1,15), (40,39,1,15), (40,49,1,15), (40,51,1,15), (40,53,1,15), (40,59,1,15), (40,61,1,15), (41,3,3,15), (41,9,2,15), (41,12,3,15), (41,25,1,15), (41,32,1,15), (41,38,1,15), (41,49,3,15), (41,53,2,15), (41,58,3,15), (42,3,1,15), (42,5,1,15), (42,8,3,15), (42,12,1,15), (42,14,36,15), (42,51,1,15), (42,53,1,15), (42,55,1,15), (42,58,1,15), (42,60,1,15), (43,3,1,15), (43,5,1,15), (43,8,1,15), (43,10,5,15), (43,26,1,15), (43,29,6,15), (43,37,1,15), (43,49,4,15), (43,54,1,15), (43,58,1,15), (43,60,1,15), (44,4,1,15), (44,6,1,15), (44,12,1,15), (44,14,1,15), (44,27,1,15), (44,37,1,15), (44,49,1,15), (44,51,1,15), (44,55,1,15), (44,57,1,15), (44,59,1,15), (45,4,1,15), (45,6,1,15), (45,12,1,15), (45,15,1,15), (45,28,1,15), (45,36,1,15), (45,48,1,15), (45,51,1,15), (45,57,1,15), (45,59,1,15), (46,5,1,15), (46,7,1,15), (46,13,1,15), (46,16,1,15), (46,28,1,15), (46,35,1,15), (46,47,1,15), (46,50,2,15), (46,56,1,15), (46,58,1,15), (47,5,1,15), (47,7,2,15), (47,10,3,15), (47,14,1,15), (47,17,1,15), (47,29,1,15), (47,35,1,15), (47,46,1,15), (47,49,1,15), (47,51,1,15), (47,55,2,15), (47,58,1,15), (48,6,1,15), (48,8,1,15), (48,12,1,15), (48,15,1,15), (48,18,1,15), (48,30,1,15), (48,34,1,15), (48,45,1,15), (48,48,1,15), (48,50,1,15), (48,55,1,15), (48,57,1,15), (49,7,1,15), (49,9,1,15), (49,11,1,15), (49,16,1,15), (49,19,1,15), (49,30,1,15), (49,34,1,15), (49,44,1,15), (49,47,1,15), (49,52,1,15), (49,54,1,15), (49,56,1,15), (50,7,1,15), (50,10,1,15), (50,15,1,15), (50,17,1,15), (50,20,2,15), (50,31,1,15), (50,33,1,15), (50,42,2,15), (50,46,1,15), (50,53,1,15), (50,56,1,15), (51,8,1,15), (51,11,1,15), (51,16,4,15), (51,22,2,15), (51,31,3,15), (51,40,2,15), (51,44,2,15), (51,47,2,15), (51,52,1,15), (51,55,1,15), (52,9,1,15), (52,11,2,15), (52,14,1,15), (52,20,1,15), (52,24,3,15), (52,31,1,15), (52,33,1,15), (52,37,3,15), (52,43,1,15), (52,47,1,15), (52,49,1,15), (52,51,2,15), (52,54,1,15), (53,10,1,15), (53,13,1,15), (53,21,3,15), (53,27,10,15), (53,40,4,15), (53,47,1,15), (53,50,1,15), (53,53,1,15), (54,11,1,15), (54,14,1,15), (54,20,1,15), (54,22,1,15), (54,24,3,15), (54,37,3,15), (54,41,2,15), (54,49,1,15), (54,52,1,15), (55,12,1,15), (55,15,2,15), (55,19,1,15), (55,21,1,15), (55,25,12,15), (55,38,1,15), (55,42,2,15), (55,47,2,15), (55,51,1,15), (56,13,2,15), (56,16,2,15), (56,26,2,15), (56,30,1,15), (56,32,1,15), (56,38,1,15), (56,46,2,15), (56,49,2,15), (57,15,1,15), (57,18,2,15), (57,26,1,15), (57,32,1,15), (57,37,2,15), (57,44,2,15), (57,48,1,15), (58,16,2,15), (58,20,3,15), (58,32,1,15), (58,36,1,15), (58,41,3,15), (58,46,2,15), (59,18,2,15), (59,22,5,15), (59,37,5,15), (59,44,2,15), (60,20,3,15), (60,27,10,15), (60,41,3,15), (61,23,4,15), (61,28,1,15), (61,35,1,15), (61,37,4,15), (62,27,10,15)];
// FX-DATA-END

// ------------------------------------------------------------------ the crafter

/// The crafter is worked like the crafting table: its nine windows are the cells on its top (the
/// same 2-pixel openings 3 pixels apart), what lies in them stands on the window, a shut window
/// has a lid over it, and what it would craft floats over the far edge.
pub fn is_crafter() -> bool {
    state().kind == Kind::Crafter
}

/// The panel's slot or a window (the crafter's slots, 300 on); the floating result is not taken.
pub fn crafter_slot_at(w: f32, h: f32, x: f32, y: f32, seconds: f64) -> Option<usize> {
    let e = expanded();
    if inside(panel(w, e), x, y) {
        return panel_slot(w, x, y, e);
    }
    match hover(ndc((x, y), w, h), seconds)? {
        Hover::Cell(i) => Some(300 + i),
        _ => None,
    }
}

pub const CRAFTER_SHUT: &str = "This window is shut";
pub const CRAFTER_OPEN_IT: &str = "Click with an empty hand \u{2014} open it";
pub const CRAFTER_SHUT_IT: &str = "Click with an empty hand \u{2014} shut this window";

/// A shut window's lid: a light plate over the cell, lit on its far and left edges, shaded on the
/// near and right ones (the crafter's disabled slot).
pub fn lid_quads(t: [i32; 3], yaw: f32, i: usize) -> Vec<([DVec3; 4], u32)> {
    let c = cell_center(t, yaw, i) + DVec3::Y * 0.004;
    let (r, f) = (right(yaw), forward(yaw));
    let a = 1.42 * PX;
    let w = 0.22 * PX;
    let q = |x0: f64, z0: f64, x1: f64, z1: f64, lift: f64| {
        let o = c + DVec3::Y * lift;
        [o + r * x0 + f * z0, o + r * x1 + f * z0, o + r * x1 + f * z1, o + r * x0 + f * z1]
    };
    let rgb = |v: [u8; 3]| u32::from_le_bytes([v[0], v[1], v[2], 255]);
    vec![
        (q(-a, -a, a, a, 0.0), rgb([194, 194, 194])),
        // Far edge and left edge: the light rim.
        (q(-a, a - w, a, a, 0.0005), rgb([240, 236, 238])),
        (q(-a, -a, -a + w, a, 0.0005), rgb([240, 236, 238])),
        // Near edge and right edge: the shade.
        (q(-a + w, -a, a, -a + w, 0.0005), rgb([132, 130, 131])),
        (q(a - w, -a + w, a, a - w, 0.0005), rgb([132, 130, 131])),
    ]
}

// ------------------------------------------------------------------ the stonecutter

/// The stone lies by the saw: on the top, left of the blade and a little towards the player.
pub const STONECUTTER_SPOTS: [Spot; 1] = [Spot { slot: 0, x: -0.31, z: -0.08, r: 0.16, y: -0.4375 }];

/// How many shapes stand round the blade at once (the wheel shows the next ones).
pub const STONE_PAGE: usize = 12;
/// The ring of shapes: round the blade's hub, in the plane that faces the camera.
pub const STONE_RING: f64 = 0.62;
/// The angle between two shapes on the ring (degrees), and the size of a shape.
pub const STONE_STEP: f64 = 16.0;
pub const STONE_SIZE: f32 = 0.19;
pub const STONE_CUT: &str = "LMB \u{2014} cut one  \u{b7}  Shift \u{2014} cut the whole stack";
pub const STONE_BUSY: &str = "Your hand holds something else \u{2014} put it away first";

pub fn stone_page() -> usize {
    state().stone_page
}

/// The wheel over the stonecutter: the next (or the previous) shapes, `total` in all.
pub fn stone_scroll(delta: f32, total: usize) {
    let mut s = state();
    if total <= STONE_PAGE {
        s.stone_page = 0;
        return;
    }
    let last = total - STONE_PAGE;
    s.stone_page = if delta < 0.0 { (s.stone_page + 4).min(last) } else { s.stone_page.saturating_sub(4) };
}

/// The shapes shown: the indices into the menu's recipe list, from the page on.
pub fn stone_shown(total: usize) -> std::ops::Range<usize> {
    let mut s = state();
    if s.stone_page >= total {
        s.stone_page = 0;
    }
    s.stone_page..(s.stone_page + STONE_PAGE).min(total)
}

/// Where the `k`th of `n` shapes floats: on a ring round the blade's hub (just under the top's
/// height) in the plane facing the camera, from the left over the top to the right, swaying a little.
pub fn stone_point(t: [i32; 3], yaw: f32, k: usize, n: usize, seconds: f64) -> DVec3 {
    let hub = top_center(t) - DVec3::Y * (0.4375 + 0.045);
    let pitch = Kind::Stonecutter.rig();
    let tilt = (pitch.up - pitch.look_up).atan2(pitch.back - pitch.look_back);
    let up = DVec3::Y * tilt.cos() + forward(yaw) * tilt.sin();
    let span = STONE_STEP * (n.max(1) - 1) as f64;
    let a = (90.0 + span * 0.5 - STONE_STEP * k as f64).to_radians();
    let sway = 0.012 * (seconds * 1.3 + k as f64 * 0.7).sin();
    hub + right(yaw) * (STONE_RING * a.cos()) + up * (STONE_RING * a.sin() + sway)
}

/// The shape under the cursor (its place among the `n` shown), the nearest within reach.
pub fn stone_at(n: (f32, f32), shown: usize, seconds: f64) -> Option<usize> {
    let (pose, t, yaw, lens) = {
        let s = state();
        if s.phase != Phase::Inside || s.kind != Kind::Stonecutter {
            return None;
        }
        (s.last?, s.table, s.yaw, s.lens)
    };
    let mut best: Option<(f32, usize)> = None;
    for k in 0..shown {
        let Some(p) = project(pose, stone_point(t, yaw, k, shown, seconds), lens.0, lens.1) else {
            continue;
        };
        let (dx, dy) = ((p.0 - n.0) * lens.1, p.1 - n.1);
        let d = dx * dx + dy * dy;
        if d < 0.085 * 0.085 && best.is_none_or(|b| d < b.0) {
            best = Some((d, k));
        }
    }
    best.map(|b| b.1)
}

/// The hand that reaches for a shape, and where it stops: a little short of it.
pub fn stone_reach(t: [i32; 3], yaw: f32, k: usize, n: usize, seconds: f64) -> (usize, DVec3) {
    let p = stone_point(t, yaw, k, n, seconds);
    let side = usize::from((p - top_center(t)).dot(right(yaw)) >= 0.0);
    (side, p - forward(yaw) * 0.09 - DVec3::Y * 0.03)
}

/// The arms at the stonecutter: resting, or one reaching for the shape under the cursor.
pub fn step_stone_hands(target: Option<(usize, DVec3)>, now: Instant, seconds: f64) -> Option<[[DVec3; 2]; 2]> {
    let (t, yaw) = {
        let s = state();
        (s.table, s.yaw)
    };
    let sway = [
        DVec3::new(0.0, 0.003 * (seconds * 1.7).sin(), 0.0),
        DVec3::new(0.0, 0.003 * (seconds * 1.5 + 2.0).sin(), 0.0),
    ];
    let mut targets = [surface_rest(Kind::Stonecutter, 0, t, yaw) + sway[0], surface_rest(Kind::Stonecutter, 1, t, yaw) + sway[1]];
    let reaching = target.map(|(side, p)| {
        targets[side] = p;
        side
    });
    step_hands_to(targets, reaching, now)
}

// ------------------------------------------------------------------ the anvil's new name

pub fn renaming() -> bool {
    state().renaming
}

pub fn set_renaming(on: bool) {
    state().renaming = on;
}

pub fn set_name_rect(rect: Option<[f32; 4]>) {
    state().name_rect = rect;
}

/// The cursor is over the anvil's floating name.
pub fn on_name(x: f32, y: f32) -> bool {
    state().name_rect.is_some_and(|r| inside(r, x, y))
}

pub const NAME_CLICK: &str = "Click \u{2014} give it a new name";
pub const NEW_NAME: &str = "New name";

/// The rename box: low in the middle of the screen, over the hints.
pub fn name_box(w: f32, h: f32) -> [f32; 4] {
    [(w * 0.5 - 76.0).floor(), (h - 53.0).floor(), 152.0, 22.0]
}

// ------------------------------------------------------------------ the stations' tooltips

/// The jar's line under the item on the worked spot (gold: what a click does).
pub fn work_effect(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Grindstone => Some("\u{a7}dEnchantments come off (curses stay), a little experience returns"),
        _ => None,
    }
}

pub const TAKE_BACK: &str = "Click to take it back";

pub fn brewing_fuel_text(fuel: i32) -> String {
    format!("Fuel left for {fuel} brews")
}

/// Whether the station's tooltip puts an empty line between the item and the station's lines.
pub fn tooltip_gap(kind: Kind) -> bool {
    !matches!(kind, Kind::Brewing | Kind::Enchanting | Kind::Beacon | Kind::Stonecutter)
}
