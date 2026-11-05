// AbstractContainerMenu.removed on the server, for a crafting table: the jar's table keeps what
// lies on it. The grid's plain stacks stay on the table (kept with the world); anything else, and
// any world without a folder, goes back to the player as vanilla gives it.
fn cinematic_table_close(game: &mut crate::game::Game<'_>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::is_active() || game.remote.is_some() || game.craft_size != 3 {
        return;
    }
    let Some(pos) = game.craft_table else {
        return;
    };
    let root = game.world_root.display().to_string();
    let dim = format!("{:?}", game.dim);
    let mut kept: [Option<(String, u32)>; 9] = Default::default();
    for (i, cell) in game.craft_grid.iter().enumerate().take(9) {
        if let Some(s) = cell
            && s.damage == 0
            && s.extra.is_none()
        {
            kept[i] = Some((s.item.to_string(), s.count));
        }
    }
    if kept.iter().all(Option::is_none) {
        return;
    }
    let yaw = ci::table_yaw(pos).unwrap_or(0.0);
    if ci::table_keep(&root, &dim, pos, yaw, kept.clone()) {
        for (i, k) in kept.iter().enumerate() {
            if k.is_some() {
                game.craft_grid[i] = None;
            }
        }
    }
}
