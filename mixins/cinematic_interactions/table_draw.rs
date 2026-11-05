// The world's crafting tables that keep items: the items lie on the top in their cells, as the
// scene laid them (CampfireRenderer's pass, items of the block entities).
fn cinematic_table_draw(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<horizon_client::mesher::Vertex>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    use horizon_assets::item_model::DisplayContext;
    let dim = format!("{:?}", game.dim);
    for t in ci::kept_tables() {
        if t.dim != dim || ci::scene_at(t.pos) {
            continue;
        }
        let at = glam::DVec3::new(f64::from(t.pos[0]), f64::from(t.pos[1]), f64::from(t.pos[2]));
        if at.distance_squared(camera) >= 64.0 * 64.0 || &*game.block_name(t.pos) != "minecraft:crafting_table" {
            continue;
        }
        let light = horizon_client::block_entity_render::vertex_light(game.world.light(t.pos[0], t.pos[1] + 1, t.pos[2]));
        for (i, cell) in t.items.iter().enumerate() {
            let Some((item, n)) = cell else {
                continue;
            };
            let stack = horizon_client::items::ItemStack::new(item, *n);
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let state = game.resolve_item(&stack, DisplayContext::Fixed, None, i as i32);
            let p = (ci::cell_center(t.pos, t.yaw, i) - camera).as_vec3();
            game.submit_item_world(&state, ci::item_matrix(p, t.yaw, block, 0.18, 0.0), light, out);
        }
    }
}
