// AbstractContainerScreen.findSlot, in the scene: the panel's inventory slots, the table's cells
// and the crafted item (or a furnace's openings) are the slots under the cursor, so every click
// works as in the screen.
fn cinematic_slot_at(game: &crate::game::Game<'_>, batch: &HudBatch, x: f32, y: f32) -> ::core::ops::ControlFlow<Option<usize>> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::controls() {
        return ::core::ops::ControlFlow::Continue(());
    }
    if ci::is_furnace(ci::kind()) {
        return ::core::ops::ControlFlow::Break(ci::furnace_slot_at(batch.width, batch.height, x, y));
    }
    if ci::is_surface(ci::kind()) {
        return ::core::ops::ControlFlow::Break(ci::surface_slot_at(batch.width, batch.height, x, y));
    }
    let seconds = game.started.elapsed().as_secs_f64();
    if ci::is_crafter() {
        return ::core::ops::ControlFlow::Break(ci::crafter_slot_at(batch.width, batch.height, x, y, seconds));
    }
    ::core::ops::ControlFlow::Break(ci::slot_at(batch.width, batch.height, x, y, seconds))
}
