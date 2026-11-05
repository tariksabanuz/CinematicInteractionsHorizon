// The server's tick: the world's kept tables are read once; a kept table that is no longer there
// (broken, in a loaded chunk) drops what lay on it, as a container's block does.
fn cinematic_table_tick(game: &mut crate::game::Game<'_>, paused: bool) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if paused || game.remote.is_some() {
        return;
    }
    let root = game.world_root.display().to_string();
    ci::tables_load(&root);
    let dim = format!("{:?}", game.dim);
    for t in ci::kept_tables() {
        if t.dim != dim || !game.world.loaded(t.pos[0], t.pos[2]) || &*game.block_name(t.pos) == "minecraft:crafting_table" {
            continue;
        }
        if let Some(items) = ci::table_take(&root, &dim, t.pos) {
            for (item, n) in items.into_iter().flatten() {
                game.pop_resource(t.pos, horizon_client::items::ItemStack::new(&item, n));
            }
        }
    }
}
