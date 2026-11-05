// At the table a step (W, A, S, D) or a jump leaves it, as Esc and E do.
fn cinematic_keys(game: &mut crate::game::Game<'_>, key: KeyCode) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    // Esc in the anvil's rename box closes the box, not the scene.
    if key == KeyCode::Escape && ci::renaming() {
        ci::set_renaming(false);
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    // H at a campfire: sit by the fire (CampfireHint; the jar's key.cinematic.sit).
    if key == KeyCode::KeyH && ci::is_active() && game.screen == crate::game::Screen::Playing && !game.spectator() {
        let Some(hit) = game.pick() else {
            return ::core::ops::ControlFlow::Continue(());
        };
        if !ci::is_campfire(&game.block_name(hit.pos)) {
            return ::core::ops::ControlFlow::Continue(());
        }
        let Some(p) = game.player.as_ref().map(|p| glam::DVec3::new(p.position.x, p.position.y, p.position.z)) else {
            return ::core::ops::ControlFlow::Continue(());
        };
        let now = std::time::Instant::now();
        if let Some(message) = ci::reach(p, hit.pos) {
            ci::toast(message, now);
            return ::core::ops::ControlFlow::Break(Ok(()));
        }
        let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        let facing = game.pack.assets.registry.state(state).and_then(|s| s.properties.get("facing")).map_or(0, |f| ci::facing_2d(f));
        ci::sit(hit.pos, facing, p, now);
        game.screen = crate::game::Screen::Inventory;
        game.capture(false);
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    if !ci::controls() || game.screen != crate::game::Screen::Inventory || game.recipe_search_focused() {
        return ::core::ops::ControlFlow::Continue(());
    }
    if !matches!(key, KeyCode::KeyW | KeyCode::KeyA | KeyCode::KeyS | KeyCode::KeyD | KeyCode::Space) {
        return ::core::ops::ControlFlow::Continue(());
    }
    game.close_container();
    game.screen = crate::game::Screen::Playing;
    game.capture(true);
    ::core::ops::ControlFlow::Break(Ok(()))
}
