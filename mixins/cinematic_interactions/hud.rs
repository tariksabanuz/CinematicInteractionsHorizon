// Hud: while the scene shows, the game's HUD (hotbar, crosshair, chat) steps aside, as F1 hides it;
// over everything come the black bars of the camera's glide and the range message.
fn cinematic_hud(game: &mut crate::game::Game<'_>, size: [u32; 2]) -> ::core::ops::ControlFlow<HudBatch> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::is_active() || ci::enter(4) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let shown = ci::shown() && !ci::walking(std::time::Instant::now());
    let hide = game.ui.hide_gui;
    if shown {
        game.ui.hide_gui = true;
    }
    let mut batch = game.hud_layers(size);
    game.ui.hide_gui = hide;
    ci::leave(4);
    // Looking at a campfire within reach: "[H] Sit by the fire" under the crosshair.
    if !ci::shown() && ci::config().invite && game.screen == crate::game::Screen::Playing && !game.ui.hide_gui && !game.spectator()
        && let Some(hit) = game.pick()
        && ci::is_campfire(&game.block_name(hit.pos))
        && game.player.as_ref().is_some_and(|p| ci::reach(glam::DVec3::new(p.position.x, p.position.y, p.position.z), hit.pos).is_none())
    {
        let (w, h) = (batch.width, batch.height);
        ci::draw_sit_plate(&mut batch, &game.gui.atlas, w, h);
    }
    ci::draw_overlay(&mut batch, &game.gui.atlas, std::time::Instant::now());
    ::core::ops::ControlFlow::Break(batch)
}
