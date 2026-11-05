// MultiPlayerGameMode.useItemOn on a crafting table or another workstation: the block out of reach
// says so ("Too low"), a sneak click with an empty hand opens the normal screen, any other click
// the scene (the camera walks to the block and leans over it).
fn cinematic_use(game: &mut crate::game::Game<'_>, hand: Hand) -> ::core::ops::ControlFlow<bool> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if hand != Hand::Main || !ci::is_active() || game.spectator() || ci::own_use() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some(hit) = game.pick() else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let low = game.player.as_ref().is_some_and(|p| p.position.y >= f64::from(hit.pos[1]) + 0.9);
    let Some(kind) = ci::kind_of(&game.block_name(hit.pos), low) else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let sneaking = game.keys.contains(&KeyCode::ShiftLeft);
    let empty = game.stack_in(game.selected).is_none() && game.offhand.is_none();
    if sneaking && !empty {
        return ::core::ops::ControlFlow::Continue(());
    }
    let now = std::time::Instant::now();
    if !sneaking
        && let Some(p) = game.player.as_ref()
        && let Some(message) = ci::reach(glam::DVec3::new(p.position.x, p.position.y, p.position.z), hit.pos)
    {
        if ci::config().reach_why {
            ci::toast(message, now);
        }
        return ::core::ops::ControlFlow::Break(true);
    }
    ci::clicked_kind(kind, hit.pos, sneaking, now);
    // A furnace is worked from its front, whichever side the player clicked it from (and so is a
    // chiseled bookshelf, which has no menu: the scene opens the inventory and stays).
    if ci::is_furnace(kind) || kind == ci::Kind::Shelf {
        let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        let facing = game.pack.assets.registry.state(state).and_then(|s| s.properties.get("facing")).and_then(|f| ci::yaw_facing(f));
        ci::click_yaw(facing);
    }
    if kind == ci::Kind::Shelf {
        if sneaking {
            return ::core::ops::ControlFlow::Continue(());
        }
        ci::hold_scene();
        game.screen = crate::game::Screen::Inventory;
        game.capture(false);
        return ::core::ops::ControlFlow::Break(true);
    }
    ::core::ops::ControlFlow::Continue(())
}
