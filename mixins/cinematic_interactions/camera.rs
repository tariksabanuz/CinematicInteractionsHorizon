// Camera.setup: at a crafting table or another workstation the eye glides over it and stays there; leaving, it glides back.
fn cinematic_camera(game: &mut crate::game::Game<'_>, partial: f32) -> ::core::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::is_active() || ci::enter(1) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let live = game.camera_view(partial);
    ci::leave(1);
    let crafting = game.remote_menu_type() == Some("minecraft:crafting") || (game.remote.is_none() && game.craft_table.is_some());
    let station = game.station.is_some() || game.container.is_some() || game.remote_menu_type().is_some_and(|m| m != "minecraft:crafting" && m != "inventory");
    // Sitting by a campfire: the inventory the H key opened; closing it stands up.
    if ci::sitting() && game.screen != crate::game::Screen::Inventory {
        ci::stand_up();
    }
    let open = game.screen == crate::game::Screen::Inventory && ((game.craft_size == 3 && crafting) || station || ci::sitting());
    let player = game.player.as_ref().map(|p| glam::DVec3::new(p.position.x, p.position.y, p.position.z));
    let now = std::time::Instant::now();
    let shown = ci::camera(open, game.craft_table, live, player, game.yaw, now);
    // The walk is done: the player now stands at the table, facing it (what the camera showed).
    if let Some((feet, yaw)) = ci::take_arrival(now) {
        let (bx, by, bz) = (feet.x.floor() as i32, (feet.y + 0.01).floor() as i32, feet.z.floor() as i32);
        let facts = game.res.facts();
        let clear = !facts.get(game.world.block(bx, by, bz)).solid_render && !facts.get(game.world.block(bx, by + 1, bz)).solid_render;
        if clear && let Some(p) = game.player.as_mut() {
            p.position.x = feet.x;
            p.position.z = feet.z;
            p.yaw = yaw;
            p.pitch = ci::WALK_PITCH;
            game.yaw = yaw;
            game.pitch = ci::WALK_PITCH;
        }
    }
    ::core::ops::ControlFlow::Break(shown.or(live))
}
