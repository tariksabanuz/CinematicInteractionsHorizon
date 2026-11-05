// A station with no menu (the campfire, the chiseled bookshelf), once a tick: what the scene's hand
// did there is the game's own use of the block (CampfireBlock and ChiseledBookShelfBlock
// .useItemOn). A carried stack is in the hand for that moment, then back on the cursor.
fn cinematic_campfire_tick(game: &mut crate::game::Game<'_>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    use horizon_client::menu::Input;
    if let Some(shift) = ci::forge_due() {
        let input = if shift { Input::QuickMove } else { Input::Pickup };
        game.menu_click(Some(650), 0, input);
    }
    if let Some(n) = ci::take_move_to() {
        let aim = game.forced_hit.take();
        game.forced_hit = Some(crate::game::Hit { pos: n, face: [0, 1, 0], point: [f64::from(n[0]) + 0.5, f64::from(n[1]) + 1.0, f64::from(n[2]) + 0.5] });
        game.remote_use_block(horizon_client::first_person::Hand::Main);
        game.forced_hit = aim;
    }
    let Some((t, at, with_carried)) = ci::take_place() else {
        return;
    };
    let carried = if with_carried { game.carried.clone() } else { None };
    if with_carried && carried.is_none() {
        return;
    }
    let slot = game.selected;
    let swap = carried.as_ref().is_some_and(|c| !game.inventory.get(slot).cloned().flatten().is_some_and(|s| s.item == c.item));
    if swap {
        game.menu_ops(&[(Some(slot), 0, Input::Pickup)]);
    }
    let (point, face) = at.unwrap_or(([f64::from(t[0]) + 0.5, f64::from(t[1]) + 0.4375, f64::from(t[2]) + 0.5], [0, 1, 0]));
    let aim = game.forced_hit.take();
    game.forced_hit = Some(crate::game::Hit { pos: t, face, point });
    ci::set_own_use(true);
    game.remote_use_block(horizon_client::first_person::Hand::Main);
    ci::set_own_use(false);
    game.forced_hit = aim;
    if swap {
        game.menu_ops(&[(Some(slot), 0, Input::Pickup)]);
    }
}
