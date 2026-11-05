// The "Cinematic Interaction Settings..." button on the Options screen: where it was laid out
// last, for the mouse hook, and the guard of the layout hook's own call of the layout.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static BUTTON: Mutex<Option<[f32; 4]>> = Mutex::new(None);
static IN_LAYOUT: AtomicBool = AtomicBool::new(false);

pub const LABEL: &str = "Cinematic Interaction Settings...";

/// True when the layout hook is not inside its own call already (and now is).
pub fn layout_enter() -> bool {
    !IN_LAYOUT.swap(true, Ordering::Relaxed)
}

pub fn layout_leave() {
    IN_LAYOUT.store(false, Ordering::Relaxed);
}

pub fn set_button(rect: Option<[f32; 4]>) {
    *BUTTON.lock().unwrap_or_else(|e| e.into_inner()) = rect;
}

/// Whether (`x`, `y`) is on the button where it was last laid out.
pub fn button_at(x: f32, y: f32) -> bool {
    BUTTON
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|r| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3])
}

/// Where the button goes among the Options screen's widgets (x, y, w of each), in a `w` by `h`
/// screen: the free gap above the grid of buttons, else beside a lone button in the grid's last
/// row, else a new row (the grid and what is below it, down to Done, moving down by one). The
/// rows to move down, from that y, and the button's rect.
pub fn place(widgets: &[[f32; 3]], w: f32, h: f32) -> Option<(Option<f32>, [f32; 4])> {
    let grid: Vec<[f32; 3]> = widgets.iter().copied().filter(|e| e[2] == 150.0 && e[1] > 40.0 && e[1] < h - 30.0).collect();
    let gy = grid.iter().map(|e| e[1]).fold(f32::INFINITY, f32::min);
    let last = grid.iter().map(|e| e[1]).fold(f32::NEG_INFINITY, f32::max);
    if !gy.is_finite() {
        return None;
    }
    let cx = (w / 2.0).floor();
    let taken = |x: f32, y: f32, bw: f32| widgets.iter().any(|e| (e[1] - y).abs() < 20.0 && e[0] < x + bw && e[0] + e[2] > x);
    for y in [gy - 24.0, gy - 48.0] {
        if y >= 53.0 && !taken(cx - 154.0, y, 308.0) {
            return Some((None, [cx - 154.0, y, 308.0, 20.0]));
        }
    }
    if !taken(cx + 4.0, last, 150.0) {
        return Some((None, [cx + 4.0, last, 150.0, 20.0]));
    }
    let lowest = widgets.iter().filter(|e| e[1] >= gy && e[1] < h - 30.0).map(|e| e[1]).fold(gy, f32::max);
    if lowest + 24.0 + 20.0 <= h - 31.0 {
        return Some((Some(gy), [cx - 154.0, gy, 308.0, 20.0]));
    }
    None
}
