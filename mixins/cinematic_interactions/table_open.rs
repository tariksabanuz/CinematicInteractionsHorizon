// A crafting table opening on the server: what was left on it is in its grid again.
fn cinematic_table_open(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if game.remote.is_some() || &*game.block_name(pos) != "minecraft:crafting_table" {
        return;
    }
    if game.craft_grid.iter().any(Option::is_some) {
        return;
    }
    let root = game.world_root.display().to_string();
    let dim = format!("{:?}", game.dim);
    let Some(items) = ci::table_take(&root, &dim, pos) else {
        return;
    };
    if game.craft_grid.len() < 9 {
        game.craft_grid.resize(9, None);
    }
    for (i, k) in items.into_iter().enumerate() {
        game.craft_grid[i] = k.map(|(item, n)| horizon_client::items::ItemStack::new(&item, n));
    }
}
