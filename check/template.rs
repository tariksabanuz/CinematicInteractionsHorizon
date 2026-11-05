// Type check of the helpers and the module against stand-ins copying the Horizon signatures they
// touch (Horizon 9d979caa). Each helper sits in its target's module, as horizon-mixins puts it.
#![allow(dead_code, unused_variables, unused_imports, unused_mut, clippy::all)]
extern crate self as horizon_client;
extern crate self as horizon_assets;
extern crate self as horizon_client_render;
extern crate self as horizon_client_lib;

pub type Result<T, E = String> = std::result::Result<T, E>;

pub mod hud {
    use std::collections::HashMap;
    #[derive(Clone, Copy, Debug)] pub struct Rect { pub u0: f32, pub v0: f32, pub u1: f32, pub v1: f32 }
    pub struct Font;
    impl Font { pub fn white(&self) -> Rect { Rect { u0: 0.0, v0: 0.0, u1: 1.0, v1: 1.0 } } pub fn width(&self, t: &str) -> f32 { t.chars().count() as f32 * 6.0 } pub fn width_text(&self, t: &crate::text::Text) -> f32 { 6.0 } }
    pub struct HudAtlas { pub sprites: HashMap<String, Rect>, pub font: Font }
    #[derive(Clone, Debug)] pub struct HudVertex;
    pub struct HudBatch { pub vertices: Vec<HudVertex>, pub scale: f32, pub width: f32, pub height: f32, pub fills: Vec<[f32; 4]> }
    impl HudBatch {
        pub fn new(window: [u32; 2]) -> Self { HudBatch { vertices: Vec::new(), scale: 4.0, width: (window[0] as f32 / 4.0).ceil(), height: (window[1] as f32 / 4.0).ceil(), fills: Vec::new() } }
        pub fn fill(&mut self, atlas: &HudAtlas, x: f32, y: f32, w: f32, h: f32, color: u32) { self.fills.push([x, y, w, h]); self.vertices.extend([HudVertex, HudVertex, HudVertex, HudVertex]); }
        pub fn text(&mut self, atlas: &HudAtlas, text: &str, x: f32, y: f32, color: u32, shadow: bool) -> f32 { self.vertices.push(HudVertex); atlas.font.width(text) }
        pub fn text_width(atlas: &HudAtlas, text: &str) -> f32 { atlas.font.width(text) }
        pub fn fill_quad(&mut self, atlas: &HudAtlas, corners: [[f32; 2]; 4], color: u32) { self.vertices.push(HudVertex); }
        pub fn text_styled(&mut self, atlas: &HudAtlas, text: &crate::text::Text, x: f32, y: f32, color: u32, shadow: bool) -> f32 { self.vertices.push(HudVertex); 0.0 }
        pub fn sprite(&mut self, atlas: &HudAtlas, name: &str, x: f32, y: f32) {}
        pub fn sprite_scaled(&mut self, atlas: &HudAtlas, name: &str, x: f32, y: f32, w: f32, h: f32, color: u32) {}
        pub fn scale_since(&mut self, start: usize, cx: f32, cy: f32, sx: f32, sy: f32) {}
        pub fn clip_since(&mut self, start: usize, x0: f32, y0: f32, x1: f32, y1: f32) {}
    }
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 { u32::from_le_bytes([r, g, b, a]) }
}
pub mod camera {
    pub fn view_matrix(yaw: f32, pitch: f32) -> glam::Mat4 {
        let q = glam::Quat::from_euler(glam::EulerRot::YXZ, std::f32::consts::PI - yaw.to_radians(), -pitch.to_radians(), 0.0);
        glam::Mat4::from_quat(glam::Quat::from_xyzw(-q.x, -q.y, -q.z, q.w))
    }
}
pub mod mesher {
    #[derive(Clone, Copy, Debug)]
    pub struct Vertex { pub pos: [f32; 3], pub uv: [f32; 2], pub layer: u32, pub color: u32, pub light: u32, pub normal: [i8; 4], pub glint_uv: [f32; 2] }
}
pub mod gfx { pub const OIT_ENTITY: u32 = 1 << 30; }
pub mod block_entity_render { pub fn vertex_light(packed: u8) -> u32 { packed as u32 } }
pub mod mobs {
    pub struct Polygon { pub normal: [f32; 3], pub vertices: [[f32; 5]; 4] }
    pub struct Part { pub name: String, pub pose: [f32; 9], pub polygons: Vec<Polygon>, pub children: Vec<Part> }
    pub struct Mob { pub skin: Option<String>, pub slim: bool }
    pub fn render_model(part: &Part, base: glam::Mat4, pose_for: &dyn Fn(&Part) -> [f32; 9], uv: [f32; 4], light: u32, overlay: u32, out: &mut Vec<crate::mesher::Vertex>) {}
}
pub mod items {
    use std::sync::Arc;
    #[derive(Clone, Debug)] pub struct ItemStack { pub item: Arc<str>, pub count: u32, pub damage: u32, pub extra: Option<Box<u8>> }
    impl ItemStack { pub fn new(item: &str, count: u32) -> Self { ItemStack { item: item.into(), count, damage: 0, extra: None } } }
    pub struct ItemInfo { pub block: Option<u32>, pub fuel: Option<(u8, u8)> }
    pub struct Items;
    impl Items { pub fn get(&self, id: &str) -> Option<&ItemInfo> { None } }
}
pub mod text {
    #[derive(Clone, Debug)] pub struct Text;
    impl Text { pub fn legacy(s: &str) -> Text { Text } pub fn literal(s: impl Into<String>) -> Text { Text } pub fn styled(s: impl Into<String>, style: Style) -> Text { Text } }
    #[derive(Clone, Debug, Default)] pub struct Style { pub color: Option<u32>, pub font: Option<String>, pub bold: Option<bool> }
}
pub mod item_tooltip { pub fn name_line(stack: &crate::items::ItemStack, translate: &dyn Fn(&str) -> Option<String>) -> String { String::new() } }
pub mod item_model { #[derive(Clone, Copy)] pub enum DisplayContext { Fixed, Ground } }
pub mod first_person { #[derive(Clone, Copy, PartialEq, Eq)] pub enum Hand { Main, Off } }
pub mod recipe_book {
    use std::collections::HashMap;
    #[derive(Clone, Copy)] pub enum BookType { Crafting } impl BookType { pub fn index(self) -> usize { 0 } }
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub enum Category { CraftingBuilding, CraftingRedstone, CraftingEquipment, CraftingMisc, FurnaceFood }
    #[derive(Clone, Debug)] pub struct Entry { pub id: String, pub display_id: i32, pub category: Category, pub result: crate::items::ItemStack }
    impl Entry { pub fn fits(&self, w: usize, h: usize) -> bool { true } }
    #[derive(Default)] pub struct Book { pub entries: Vec<Entry>, pub placements: HashMap<String, crate::crafting::Placement> }
    impl Book { pub fn can_craft(&self, entry: &Entry, stock: &crate::menu::place::Stacked) -> bool { false } }
}
pub mod crafting {
    pub struct Placement { pub slots: Vec<i32> }
    impl Placement { pub fn impossible(&self) -> bool { self.slots.is_empty() } }
    pub struct Ingredient(pub Vec<String>);
    impl Ingredient { pub fn test(&self, stack: &crate::items::ItemStack) -> bool { self.0.iter().any(|i| i.as_str() == &*stack.item) } }
    pub struct Cooking { pub id: String, pub kind: String, pub ingredient: Ingredient }
    pub struct Recipes { pub cooking: Vec<Cooking> }
    impl Recipes { pub fn cook(&self, kind: &str, input: &crate::items::ItemStack) -> Option<&Cooking> { None } }
}
pub mod menu {
    pub struct Stonecutter { pub recipes: Vec<usize>, pub input: Option<std::sync::Arc<str>>, pub used: Option<usize> }
    pub struct Menu;
    pub struct Env;
    impl Menu { pub fn button(&mut self, env: &Env, id: i32) -> bool { true } }
    #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Input { Pickup, QuickMove, Swap, Clone, Throw, QuickCraft, PickupAll }
    pub mod place { pub struct Stacked; }
}
pub mod containers {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum FurnaceKind { Furnace, BlastFurnace, Smoker }
    #[derive(Clone, Debug)] pub struct Furnace { pub kind: FurnaceKind, pub items: [Option<crate::items::ItemStack>; 3], pub lit_remaining: i32, pub lit_total: i32, pub cooking_timer: i32, pub cooking_total: i32, pub speed: f32 }
    #[derive(Clone, Debug)] pub struct Beacon { pub levels: i32, pub primary: Option<String>, pub secondary: Option<String> }
    #[derive(Clone, Debug)] pub enum BlockEntity { Chest(Vec<Option<crate::items::ItemStack>>), Furnace(Furnace), Beacon(Box<Beacon>), Campfire(Box<Campfire>), ChiseledBookshelf(Box<ChiseledBookshelf>), Crafter(Crafter), Brewing(Box<BrewingStand>) }
    #[derive(Clone, Debug, Default)] pub struct Crafter { pub items: [Option<crate::items::ItemStack>; 9], pub disabled: [bool; 9], pub triggered: bool, pub crafting_ticks: i32 }
    #[derive(Clone, Debug, Default)] pub struct BrewingStand { pub items: [Option<crate::items::ItemStack>; 5], pub brew_time: i32, pub fuel: i32 }
    #[derive(Clone, Debug, Default)] pub struct ChiseledBookshelf { pub items: [Option<crate::items::ItemStack>; 6], pub last_slot: i32 }
    #[derive(Clone, Debug, Default)] pub struct Campfire { pub items: [Option<crate::items::ItemStack>; 4], pub progress: [i32; 4], pub total: [i32; 4] }
}
pub mod container_screen { pub struct Clicks; impl Clicks { pub fn skip_release(&mut self) {} } }

pub mod ui {
    use crate::hud::{HudAtlas, HudBatch};
    use crate::text::Text;
    pub struct EditBox { pub value: String, pub cursor: usize, pub highlight: usize, pub focused: bool }
    impl EditBox { pub fn set_value(&mut self, value: &str) {} pub fn cursor_visible(&self) -> bool { true } pub fn reset_blink(&mut self) {} }
    pub fn button(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, w: f32, h: f32, label: &Text, active: bool, hover: bool) {}
    pub fn slider(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, w: f32, label: &Text, value: f32, hover: bool, active: bool, dragging: bool) {}
    pub fn slider_value(x: f32, w: f32, cx: f32) -> f32 { 0.0 }
    pub fn centered_text(batch: &mut HudBatch, atlas: &HudAtlas, text: &Text, x: f32, y: f32, w: f32, color: u32) {}
    pub fn separators(batch: &mut HudBatch, atlas: &HudAtlas, header_bottom: f32, footer_top: f32, in_world: bool) {}
    pub fn scrollbar(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, top: f32, bottom: f32, content: f32, scroll: f32) {}
}
pub mod options_ui {
    use crate::text::Text;
    #[derive(Clone, Copy, PartialEq, Eq)] pub enum Page { Main, Other }
    pub enum Event { ButtonClick }
    pub enum Act { None }
    pub struct Settings;
    pub struct Widget { pub x: f32, pub y: f32, pub w: f32 }
    #[derive(Default)] pub struct OptionsUi;
    impl OptionsUi { pub fn page(&self) -> Page { Page::Main } pub fn layout(&mut self, s: &Settings, w: f32, h: f32) -> (Vec<Widget>, f32, f32, f32) { (Vec::new(), 0.0, 0.0, 0.0) } }
    pub fn button(x: f32, y: f32, w: f32, label: Text, act: Act) -> Widget { Widget { x, y, w } }
    //OPTIONS_LAYOUT//
}
pub mod native_cinematic_menu {
//MENU//
}
pub mod native_cinematic_button {
//BUTTON//
}
pub mod native_cinematic_interactions_scene {
//SCENE//
}

pub mod game {
    use crate::hud::HudBatch;
    use crate::items::{ItemStack, Items};
    use crate::mesher::Vertex;
    use crate::first_person::Hand;
    use crate::Result;
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub enum KeyCode { KeyW, KeyA, KeyS, KeyD, Space, ShiftLeft, KeyE, KeyH, Escape, Backspace }
    #[derive(Clone, Copy, PartialEq, Eq)] pub(crate) enum Screen { Playing, Inventory }
    #[derive(Clone, Copy)] pub(crate) struct Hit { pub(crate) pos: [i32; 3], pub(crate) face: [i32; 3], pub(crate) point: [f64; 3] }
    pub struct V3 { pub x: f64, pub y: f64, pub z: f64 }
    pub struct PlayerMotion { pub position: V3, pub yaw: f32, pub pitch: f32 }
    pub struct Config { pub width: u32, pub height: u32 }
    pub struct Renderer { pub config: Config }
    pub struct UiState { pub hide_gui: bool, pub options: crate::options_ui::OptionsUi }
    pub struct UiResources { pub atlas: crate::hud::HudAtlas }
    pub struct Resources;
    impl Resources { pub(crate) fn player_arm(&self, slim: bool, left: bool) -> &Option<crate::mobs::Part> { &None } }
    pub struct World { pub block_entities: std::collections::HashMap<(i32, i32, i32), crate::containers::BlockEntity> }
    #[derive(Clone, Copy)] pub struct BlockStateId(pub u32);
    impl World { pub fn loaded(&self, x: i32, z: i32) -> bool { true } pub fn light(&self, x: i32, y: i32, z: i32) -> u8 { 0 } pub fn block(&self, x: i32, y: i32, z: i32) -> BlockStateId { BlockStateId(0) } }
    pub struct BlockState { pub name: String, pub properties: std::collections::BTreeMap<String, String> }
    pub struct BlockRegistry;
    impl BlockRegistry { pub fn state(&self, id: BlockStateId) -> Option<&BlockState> { None } }
    pub struct Assets { pub registry: &'static BlockRegistry }
    pub struct Fact { pub solid_render: bool }
    pub struct Facts;
    impl Facts { pub fn get(&self, s: BlockStateId) -> &Fact { &Fact { solid_render: false } } }
    impl Resources { pub fn facts(&self) -> &Facts { &Facts } }
    pub struct Names;
    impl Names { pub fn display_get(&self, key: &str) -> Option<String> { None } }
    pub struct PackResources { pub names: Names, pub assets: std::sync::Arc<Assets> }
    pub struct Audio;
    impl Audio { pub fn play(&self, s: &str, at: Option<[f64; 3]>, v: f32, p: f32) {} }
    pub struct BookSettings { pub open: [bool; 4], pub filtering: [bool; 4] }
    pub struct Level { pub recipe_book: BookSettings, pub xp_seed: i32 }
    pub use crate::recipe_book::{Book, Entry};
    pub struct RButton { pub entries: Vec<(usize, bool)> }
    pub struct View { pub buttons: Vec<RButton> }
    pub struct EditBox { pub value: String, pub focused: bool }
    pub struct Ghost { pub(crate) slot: usize, pub(crate) items: Vec<ItemStack>, pub(crate) result: bool }
    pub struct RecipeUi { pub book: Option<Book>, pub view: View, pub page: usize, pub search: Option<EditBox>, pub ghost: Vec<Ghost> }
    pub struct ItemRenderState;
    pub struct Experience { pub level: i32 }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub(crate) enum Dim { Overworld, Nether, End }
    pub struct Enchantments;
    impl Enchantments { pub fn full_name(&self, id: &str, level: u32) -> (String, bool) { (String::new(), false) } }
    pub struct Stonecutting { pub id: String, pub result: ItemStack }
    pub struct StationRecipes { pub stonecutting: Vec<Stonecutting> }
    pub struct StationData { pub enchantments: Enchantments, pub recipes: StationRecipes }
    pub struct Remote;
    pub struct Game<'a> {
        pub(crate) screen: Screen, pub(crate) craft_size: usize, pub(crate) station: Option<stations::Station>, pub(crate) creative: bool, pub(crate) experience: Experience, pub(crate) container: Option<(i32, i32, i32)>, pub(crate) craft_table: Option<[i32; 3]>, pub(crate) remote: Option<Remote>,
        pub(crate) player: Option<PlayerMotion>, pub(crate) yaw: f32, pub(crate) renderer: Option<Renderer>, pub(crate) started: std::time::Instant,
        pub(crate) cursor: (f32, f32), pub(crate) dragging: bool, pub(crate) world: World, pub(crate) player_model: Option<crate::mobs::Mob>, pub(crate) gui: UiResources,
        pub(crate) res: std::sync::Arc<Resources>, pub(crate) craft_grid: Vec<Option<ItemStack>>, pub(crate) carried: Option<ItemStack>, pub(crate) inv_drag: Option<(i32, Vec<usize>)>,
        pub(crate) selected: usize, pub(crate) inventory: Vec<Option<ItemStack>>, pub(crate) ui: UiState, pub(crate) inv_clicks: crate::container_screen::Clicks,
        pub(crate) keys: std::collections::HashSet<KeyCode>, pub(crate) offhand: Option<ItemStack>, pub(crate) pitch: f32, pub(crate) pack: PackResources, pub(crate) audio: Audio, pub(crate) level: Level, pub(crate) recipe_ui: RecipeUi, pub(crate) forced_hit: Option<Hit>, pub(crate) world_root: std::path::PathBuf, pub(crate) dim: Dim, pub(crate) _l: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub(crate) fn camera_view(&mut self, partial: f32) -> Option<(glam::DVec3, f32, f32)> { None }
        pub(crate) fn remote_menu_type(&self) -> Option<&str> { None }
        pub(crate) fn fov_degrees(&self, partial: f32, camera: [f64; 3]) -> f32 { 70.0 }
        pub(crate) fn hud_fov_degrees(&self, partial: f32, camera: [f64; 3]) -> f32 { 70.0 }
        pub(crate) fn slot_view(&mut self, i: usize) -> Option<ItemStack> { None }
        pub(crate) fn own_invisible(&self) -> bool { false }
        pub(crate) fn items(&self) -> &Items { &Items }
        pub(crate) fn recipes(&self) -> &crate::crafting::Recipes { unimplemented!() }
        pub(crate) fn menu_ops(&mut self, ops: &[(Option<usize>, i32, crate::menu::Input)]) -> bool { true }
        pub(crate) fn quick_craft_accepts(&mut self, slot: usize, ty: i32, dragged: usize) -> bool { true }
        pub(crate) fn book_available(&mut self, b: crate::recipe_book::BookType) -> crate::menu::place::Stacked { crate::menu::place::Stacked }
        pub(crate) fn resolve_item(&self, stack: &ItemStack, display: crate::item_model::DisplayContext, owner: Option<&()>, seed: i32) -> ItemRenderState { ItemRenderState }
        pub(crate) fn submit_item_world(&self, state: &ItemRenderState, pose: glam::Mat4, light: u32, out: &mut Vec<Vertex>) {}
        pub(crate) fn slot_at(&self, batch: &HudBatch, x: f32, y: f32) -> Option<usize> { None }
        pub(crate) fn draw_item(&self, batch: &mut HudBatch, stack: &ItemStack, x: f32, y: f32) {}
        pub(crate) fn draw_carried(&mut self, batch: &mut HudBatch) {}
        pub(crate) fn draw_slot_tooltip(&mut self, batch: &mut HudBatch) {}
        pub(crate) fn hud_layers(&mut self, size: [u32; 2]) -> HudBatch { HudBatch::new(size) }
        pub(crate) fn spectator(&self) -> bool { false }
        pub(crate) fn pick(&self) -> Option<Hit> { None }
        pub(crate) fn block_name(&self, p: [i32; 3]) -> std::sync::Arc<str> { "".into() }
        pub(crate) fn stack_in(&self, slot: usize) -> Option<&ItemStack> { None }
        pub(crate) fn close_container(&mut self) {}
        pub(crate) fn capture(&mut self, capture: bool) {}
        pub(crate) fn native_on(&self, id: &str) -> bool { true }
        pub(crate) fn ghost_stack(&self, slot: usize) -> Option<ItemStack> { None }
        pub(crate) fn has_shift_down(&self) -> bool { false }
        pub(crate) fn draw_tooltip_lines<T: Clone + Into<crate::text::Text>>(&self, batch: &mut HudBatch, lines: &[(T, u32)], x: f32, y: f32) {}
        pub(crate) fn draw_recipe_book(&mut self, batch: &mut HudBatch) {}
        pub(crate) fn recipe_book_click(&mut self, right: bool) -> bool { false }
        pub(crate) fn recipe_search_focused(&self) -> bool { false }
        pub(crate) fn book_open(&self, b: crate::recipe_book::BookType) -> bool { false }
        pub(crate) fn book_filtering(&self, b: crate::recipe_book::BookType) -> bool { false }
        pub(crate) fn refresh_book_view(&mut self, b: crate::recipe_book::BookType) {}
        pub(crate) fn ensure_book(&mut self) {}
        pub(crate) fn fill_ghost(&mut self, b: crate::recipe_book::BookType, e: &Entry) {}
        pub(crate) fn place_recipe(&mut self, b: crate::recipe_book::BookType, i: usize, all: bool) {}
        pub(crate) fn send_book_settings(&self, b: crate::recipe_book::BookType) {}
        pub(crate) fn stations(&self) -> &StationData { unimplemented!() }
        pub(crate) fn translate(&self, key: &str, fallback: &str) -> String { fallback.to_owned() }
        pub(crate) fn click_enchant(&mut self, button: usize) {}
        pub(crate) fn beacon_click(&mut self, local: (f32, f32)) -> bool { false }
        pub(crate) fn station_click(&mut self, right: bool) -> bool { false }
        pub(crate) fn menu_click(&mut self, slot: Option<usize>, button: i32, input: crate::menu::Input) -> bool { true }
        pub(crate) fn remote_use_block(&mut self, hand: Hand) -> bool { false }
        pub(crate) fn tick_remote(&mut self) -> Result<()> { Ok(()) }
        pub(crate) fn draw_loom(&mut self, batch: &mut HudBatch, ox: f32, oy: f32, local: (f32, f32)) -> Option<Vec<(crate::text::Text, u32)>> { None }
        pub(crate) fn loom_click(&mut self, local: (f32, f32)) -> bool { false }
        pub(crate) fn open_menu(&mut self, pos: [i32; 3]) {}
        pub(crate) fn pop_resource(&mut self, pos: [i32; 3], stack: ItemStack) {}
        pub(crate) fn tick_server(&mut self, paused: bool) -> Result<()> { Ok(()) }
        pub(crate) fn campfire_geometry(&self, camera: glam::DVec3, out: &mut Vec<Vertex>) {}
        pub(crate) fn native_config_dir() -> Result<std::path::PathBuf, String> { Err(String::new()) }
        pub(crate) fn crafter_result(&self) -> Option<ItemStack> { None }
        pub(crate) fn stack_tooltip(&self, stack: &ItemStack) -> Vec<(crate::text::Text, u32)> { Vec::new() }
        pub(crate) fn menu_session(&mut self, run: impl FnOnce(&mut crate::menu::Menu, &crate::menu::Env, &dyn Fn(usize) -> Option<usize>) -> Option<bool>) -> bool { true }
        pub(crate) fn remote_station_button(&self, button: i32) {}
        pub(crate) fn station_changed(&mut self) {}
    }
    //HANDS//
    //SLOT_AT//
    //OUTSIDE//
    //PRESS//
    //DRAW//
    //HUD//
    //KEYS//
    //FRAME//
    //TABLE_CLOSE//
    //TABLE_OPEN//
    pub mod levels {
        //TABLE_TICK//
    }
    pub mod campfire {
        //TABLE_DRAW//
    }
    pub mod perspective {
        use crate::hud::HudBatch;
        //CAMERA//
    }
    pub mod recipe_book {
        use crate::hud::{HudBatch, rgba};
        use crate::items::ItemStack;
        use crate::recipe_book::BookType;
        use crate::text::Text;
        //BOOK_DRAW//
        //BOOK_CLICK//
    }
    pub mod stations {
        use crate::hud::{HudBatch, rgba};
        use crate::items::ItemStack;
        #[derive(Clone, Default, Debug)] pub struct EnchantOffers { pub costs: [i32; 3], pub clues: [Option<(std::sync::Arc<str>, u32)>; 3] }
        pub struct Station { pub(crate) pos: [i32; 3], pub(crate) inputs: Vec<Option<ItemStack>>, pub(crate) result: Option<ItemStack>, pub(crate) offers: EnchantOffers, pub(crate) beacon: [Option<usize>; 2], pub(crate) beacon_levels: Option<i32>, pub(crate) loom: LoomState, pub(crate) anvil: AnvilState, pub(crate) name: String, pub(crate) stone: crate::menu::Stonecutter, pub(crate) rename: crate::ui::EditBox }
        pub struct AnvilState { pub cost: i32 }
        pub struct LoomState { pub(crate) display: bool }
        //STATION_CLICK//
        //STONE_SCROLL//
        //NAME_TYPING//
        //NAME_KEY//
    }
    pub mod fov {
        //FOV//
    }
    pub mod remote {
        use crate::first_person::Hand;
        use super::KeyCode;
        //USE_TABLE//
        //CAMPFIRE_TICK//
    }
    pub mod options {
        use super::{Game, KeyCode};
        use crate::hud::HudBatch;
        use crate::Result;
        use crate::options_ui::{Event, Page};
        impl Game<'_> {
            pub(super) fn gui_size(&self) -> (f32, f32) { (0.0, 0.0) }
            pub(super) fn handle_option_events(&mut self, events: Vec<Event>) -> Result<()> { Ok(()) }
        }
        //OPTIONS_DRAW//
        //OPTIONS_MOUSE//
        //OPTIONS_DRAG//
        //OPTIONS_SCROLL//
        //OPTIONS_KEY//
    }
    pub mod native_hooks {
        pub struct NativeMod { pub id: String, pub title: String, pub description: String, pub keys: Vec<String> }
        //LISTED//
        //SETTINGS//
        //SETTINGS_CLICK//
    }
}
