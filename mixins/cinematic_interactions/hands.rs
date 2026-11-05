// FirstPersonHandsAndItemsRenderer, in the scene: both arms of the player's own skin, long and
// straight as the jar draws them, rest on the station and one reaches for what the cursor is over.
// At the table the grid's items lie on its top, the crafted item floats over its far edge and the
// tabletop guide shows faint; at a furnace the items lie in its openings and the zone under the
// cursor is framed. A carried stack is in the reaching hand.
fn cinematic_hands(game: &mut crate::game::Game<'_>, alpha: f64) -> ::core::ops::ControlFlow<(Vec<Vertex>, Vec<(u32, i32)>)> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    use horizon_assets::item_model::DisplayContext;
    let furnace = ci::is_furnace(ci::kind());
    let surface = ci::is_surface(ci::kind());
    let crafter = ci::is_crafter();
    // On the walk to the table the game's own hand shows, bobbing.
    if !ci::shown() || !(ci::table_scene() || crafter || furnace || surface) || ci::walking(std::time::Instant::now()) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some(pose) = ci::view_pose() else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let now = std::time::Instant::now();
    let seconds = game.started.elapsed().as_secs_f64();
    let a = alpha as f32;
    let cam = pose.0;
    let world_fov = game.fov_degrees(a, cam.to_array());
    let hud_fov = game.hud_fov_degrees(a, cam.to_array());
    let batch = HudBatch::new(size);
    ci::set_lens(world_fov, batch.width / batch.height.max(1.0));
    let over_panel = ci::on_panel(batch.width, game.cursor.0, game.cursor.1);
    ci::set_carrying(game.carried.is_some());
    let cursor = ci::ndc(game.cursor, batch.width, batch.height);
    let (table, yaw) = ci::table();
    let mut result = None;
    let mut hover = None;
    let mut zone = None;
    let mut spot = None;
    // The stonecutter's shapes: what each recipe of the stone makes, the page of them shown, and the
    // one under the cursor.
    let mut shapes: Vec<horizon_client::items::ItemStack> = Vec::new();
    let mut shape = None;
    if surface && ci::kind() == ci::Kind::Stonecutter
        && let Some(st) = game.station.as_ref()
        && st.inputs.first().is_some_and(|i| i.is_some())
    {
        let all: Vec<horizon_client::items::ItemStack> = st.stone.recipes.iter().filter_map(|r| game.stations().recipes.stonecutting.get(*r).map(|c| c.result.clone())).collect();
        let range = ci::stone_shown(all.len());
        shapes = all[range].to_vec();
        shape = if over_panel { None } else { ci::stone_at(cursor, shapes.len(), seconds) };
    }
    let arms = if furnace {
        zone = if over_panel { None } else { ci::furnace_hover(cursor) };
        ci::step_furnace_hands(zone, now, seconds)
    } else if surface {
        ci::set_result_shown(game.slot_view(650).is_some());
        spot = if over_panel { None } else { ci::surface_hover(cursor) };
        if ci::kind() == ci::Kind::Stonecutter && shape.is_some() {
            ci::step_stone_hands(shape.map(|k| ci::stone_reach(table, yaw, k, shapes.len(), seconds)), now, seconds)
        } else {
            ci::step_surface_hands(spot, now, seconds)
        }
    } else if crafter {
        // The crafter: its windows are the cells; what it would craft floats, not to be taken.
        result = game.crafter_result();
        ci::set_result_shown(false);
        hover = ci::hover(cursor, seconds);
        ci::step_hands(hover, now, seconds)
    } else {
        result = game.slot_view(200);
        ci::set_result_shown(result.is_some());
        hover = ci::lay_target(now).map(ci::Hover::Cell).or_else(|| ci::hover(cursor, seconds));
        ci::step_hands(hover, now, seconds)
    };
    let Some(arms) = arms else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let view = ci::view_of(pose);
    let inverse = view.inverse();
    let k = ci::fov_k(hud_fov, world_fov);
    let fix = |p: glam::DVec3| ci::fov_fix((p - cam).as_vec3(), view, inverse, k);
    // The light where the work is: over the table, in front of a furnace.
    let lit = if furnace {
        let f = ci::front_point(table, yaw, 0.5, 0.5, 0.5);
        [f.x.floor() as i32, f.y.floor() as i32, f.z.floor() as i32]
    } else {
        [table[0], table[1] + 1, table[2]]
    };
    let light = game.world.light(lit[0], lit[1], lit[2]);
    let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
    let item_light = horizon_client::block_entity_render::vertex_light(light);
    let mut out = Vec::new();
    // The arms, with the player's own skin and its sleeves.
    if !game.own_invisible() {
        let slim = game.player_model.as_ref().is_some_and(|m| m.slim);
        let sprites = &game.gui.atlas.sprites;
        let skin = game
            .player_model
            .as_ref()
            .and_then(|m| m.skin.as_deref())
            .and_then(|skin| sprites.get(skin))
            .or_else(|| sprites.get("entity/player/wide/steve"))
            .copied();
        if let Some(r) = skin {
            for (side, left) in [(0usize, true), (1usize, false)] {
                for quad in ci::arm_quads(arms[side][0], arms[side][1], left, slim) {
                    let c = (255.0 * quad.shade) as u8;
                    let color = u32::from_le_bytes([c, c, c, 255]);
                    let make = |i: usize| horizon_client::mesher::Vertex {
                        pos: fix(quad.corners[i]).to_array(),
                        uv: [r.u0 + quad.uv[i][0] / 64.0 * (r.u1 - r.u0), r.v0 + quad.uv[i][1] / 64.0 * (r.v1 - r.v0)],
                        layer: 0,
                        color,
                        light: packed,
                        normal: [0, 0, 0, 0],
                        glint_uv: [0.0, 0.0],
                    };
                    // The hand pass culls nothing: one winding shows from both sides.
                    for i in 0..4 {
                        out.push(make(i));
                    }
                }
            }
        }
    }
    let white = game.gui.atlas.font.white();
    let uv = [(white.u0 + white.u1) * 0.5, (white.v0 + white.v1) * 0.5];
    let mut quads: Vec<([glam::DVec3; 4], u32)> = Vec::new();
    let presence = ci::presence(now);
    if furnace {
        // The furnace's items, lying in its openings as the jar's beds hold them.
        if let Some(layout) = ci::furnace_layout(ci::kind()) {
            for b in layout.beds.iter() {
                let Some(stack) = game.slot_view(300 + b.slot) else {
                    continue;
                };
                let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
                let copies = ci::bed_copies(stack.count as u32, block, b.copies);
                let width = (b.u1 - b.u0) as f32;
                let size = if block { (width * 0.8).min(0.2) } else { (width / copies as f32 * 1.1).min(0.15) };
                for (n, at) in ci::bed_spots(b, table, yaw, copies).into_iter().enumerate() {
                    let state = game.resolve_item(&stack, DisplayContext::Fixed, None, 40 + n as i32);
                    let spin = if block { 0.0 } else { (n as f32 * 37.0) % 30.0 - 15.0 };
                    game.submit_item_world(&state, ci::item_matrix(fix(at), yaw, block, size * presence.max(0.001), spin), item_light, &mut out);
                }
            }
            // The zone under the cursor, framed.
            if let Some(slot) = zone
                && let Some(z) = layout.zones.iter().find(|z| z.slot == slot)
            {
                for q in ci::zone_frame(z, table, yaw) {
                    quads.push((q, u32::from_le_bytes([236, 226, 196, 255])));
                }
            }
        }
    } else if surface {
        // What lies on the smithing or cartography table, each on its spot.
        let kind = ci::kind();
        for s in ci::surface_spots(kind) {
            let Some(stack) = game.slot_view(ci::spot_base(kind) + s.slot) else {
                continue;
            };
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let size = if block { (s.r * 1.1).min(0.25) } else { (s.r * 1.45).min(0.36) } as f32;
            let state = game.resolve_item(&stack, DisplayContext::Fixed, None, 60 + s.slot as i32);
            // The enchanting table's item floats upright over the book, bobbing and turning.
            let floating = kind == ci::Kind::Enchanting && s.slot == 0;
            let matrix = if floating && !block {
                ci::item_upright(fix(ci::spot_point(table, yaw, s) + glam::DVec3::Y * ci::bob(seconds)), yaw, 0.6 * presence.max(0.001), (seconds * 45.0) as f32)
            } else if floating {
                ci::item_matrix(fix(ci::spot_point(table, yaw, s) + glam::DVec3::Y * ci::bob(seconds)), yaw, block, size * presence.max(0.001), (seconds * 45.0) as f32)
            } else {
                ci::item_matrix(fix(ci::spot_point(table, yaw, s)), yaw, block, size * presence.max(0.001), 0.0)
            };
            game.submit_item_world(&state, matrix, item_light, &mut out);
        }
        // The stonecutter's shapes, round the blade; the one under the cursor a little bigger.
        for (k, stack) in shapes.iter().enumerate() {
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let at = ci::stone_point(table, yaw, k, shapes.len(), seconds);
            let size = ci::STONE_SIZE * if shape == Some(k) { 1.15 } else { 1.0 } * presence.max(0.001);
            let spin = k as f32 * 33.0 + (seconds * 12.0) as f32;
            let state = game.resolve_item(stack, DisplayContext::Fixed, None, 80 + k as i32);
            let matrix = if block {
                ci::item_matrix(fix(at - glam::DVec3::Y * f64::from(size * 0.5)), yaw, true, size, spin)
            } else {
                ci::item_upright(fix(at), yaw, size * 1.8, spin)
            };
            game.submit_item_world(&state, matrix, item_light, &mut out);
        }
        if let Some(slot) = spot
            && shape.is_none()
            && kind != ci::Kind::Shelf
            && let Some(s) = ci::surface_spots(kind).iter().find(|s| s.slot == slot)
        {
            for q in ci::spot_frame(s, table, yaw) {
                quads.push((q, u32::from_le_bytes([236, 226, 196, 255])));
            }
        }
    } else if crafter {
        // The crafter: what lies in each open window, a lid on each shut one, the frame round the
        // window under the cursor, and what it would craft floating over the far edge.
        let disabled = match game.world.block_entities.get(&(table[0], table[1], table[2])) {
            Some(horizon_client::containers::BlockEntity::Crafter(c)) => c.disabled,
            _ => [false; 9],
        };
        for i in 0..9 {
            if disabled[i] {
                quads.extend(ci::lid_quads(table, yaw, i));
                continue;
            }
            let Some(stack) = game.slot_view(300 + i) else {
                continue;
            };
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let at = fix(ci::cell_center(table, yaw, i));
            let state = game.resolve_item(&stack, DisplayContext::Fixed, None, i as i32);
            game.submit_item_world(&state, ci::item_matrix(at, yaw, block, 0.18 * presence.max(0.001), 0.0), item_light, &mut out);
        }
        if let Some(stack) = result.as_ref()
            && ci::phase() == ci::Phase::Inside
        {
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let at = fix(ci::result_spot(table, yaw, seconds)) - glam::Vec3::Y * 0.1;
            let state = game.resolve_item(stack, DisplayContext::Fixed, None, 9);
            let m = glam::Mat4::from_translation(at + glam::Vec3::Y * 0.1)
                * glam::Mat4::from_rotation_y((seconds as f32 * 60.0).to_radians())
                * glam::Mat4::from_scale(glam::Vec3::splat(if block { 0.4 } else { 0.45 }));
            game.submit_item_world(&state, m, item_light, &mut out);
        }
        if let Some(ci::Hover::Cell(i)) = hover {
            for q in ci::frame_quads(ci::cell_center(table, yaw, i), yaw) {
                quads.push((q, u32::from_le_bytes([236, 226, 196, 255])));
            }
        }
    } else {
        // The grid's items on the table top.
        for i in 0..9 {
            let Some(stack) = game.craft_grid.get(i).cloned().flatten() else {
                continue;
            };
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let at = fix(ci::cell_center(table, yaw, i));
            let state = game.resolve_item(&stack, DisplayContext::Fixed, None, i as i32);
            let size = 0.18 * presence.max(0.001);
            game.submit_item_world(&state, ci::item_matrix(at, yaw, block, size, 0.0), item_light, &mut out);
        }
        // The tabletop guide: the chosen recipe's ingredients, faint on the empty cells, framed red
        // where the inventory has none.
        let mut faint = Vec::new();
        let mut faint_frames: Vec<(usize, bool)> = Vec::new();
        for i in 0..9 {
            if game.craft_grid.get(i).is_some_and(|c| c.is_some()) {
                continue;
            }
            let Some(stack) = game.ghost_stack(100 + i) else {
                continue;
            };
            let have = game.inventory.iter().flatten().any(|s| s.item == stack.item);
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let at = fix(ci::cell_center(table, yaw, i));
            let state = game.resolve_item(&stack, DisplayContext::Fixed, None, 20 + i as i32);
            let start = out.len();
            game.submit_item_world(&state, ci::item_matrix(at, yaw, block, 0.18 * presence.max(0.001), 0.0), item_light, &mut out);
            faint.push(start..out.len());
            faint_frames.push((i, have));
        }
        for range in faint {
            for v in &mut out[range] {
                v.color = (v.color & 0x00FF_FFFF) | (110 << 24);
            }
        }
        // The crafted item, floating and turning.
        if let Some(stack) = result.as_ref()
            && ci::phase() == ci::Phase::Inside
        {
            let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
            let big = if hover == Some(ci::Hover::Result) { 1.25 } else { 1.0 };
            let at = fix(ci::result_spot(table, yaw, seconds)) - glam::Vec3::Y * 0.1;
            let state = game.resolve_item(stack, DisplayContext::Fixed, None, 9);
            let m = glam::Mat4::from_translation(at + glam::Vec3::Y * 0.1)
                * glam::Mat4::from_rotation_y((seconds as f32 * 60.0).to_radians())
                * glam::Mat4::from_scale(glam::Vec3::splat(if block { 0.4 } else { 0.45 } * big));
            game.submit_item_world(&state, m, item_light, &mut out);
        }
        // The hovered cell's frame, and the guide's cells' (red where the ingredient is missing).
        let mut frames: Vec<(usize, u32)> = faint_frames
            .iter()
            .map(|&(i, have)| (i, if have { u32::from_le_bytes([236, 226, 196, 255]) } else { u32::from_le_bytes([226, 70, 60, 255]) }))
            .collect();
        if let Some(ci::Hover::Cell(i)) = hover {
            frames.push((i, u32::from_le_bytes([236, 226, 196, 255])));
        }
        for (i, color) in frames {
            for q in ci::frame_quads(ci::cell_center(table, yaw, i), yaw) {
                quads.push((q, color));
            }
        }
    }
    // The chiseled bookshelf's book sliding out of its place or in.
    if surface
        && ci::kind() == ci::Kind::Shelf
        && let Some((item, at)) = ci::slide_pose(table, yaw)
    {
        let stack = ItemStack::new(&item, 1);
        let state = game.resolve_item(&stack, DisplayContext::Fixed, None, 70);
        game.submit_item_world(&state, ci::item_upright(fix(at), yaw, 0.22, 90.0), item_light, &mut out);
    }
    // The anvil's hammer in the swing.
    if surface && ci::kind() == ci::Kind::Anvil {
        quads.extend(ci::hammer_quads(table, yaw));
    }
    // A carried stack is in the hand, unless the cursor is over the inventory panel.
    if let Some(stack) = game.carried.clone()
        && !over_panel
    {
        let side = ci::reaching().unwrap_or(1);
        let block = game.items().get(&stack.item).is_some_and(|info| info.block.is_some());
        let at = fix(arms[side][1] + glam::DVec3::Y * 0.05);
        let state = game.resolve_item(&stack, DisplayContext::Fixed, None, 10);
        game.submit_item_world(&state, ci::item_matrix(at, yaw, block, 0.17, 0.0), item_light, &mut out);
    }
    for (quad, color) in quads {
        for corner in 0..4 {
            out.push(horizon_client::mesher::Vertex {
                pos: fix(quad[corner]).to_array(),
                uv,
                layer: 0,
                color,
                light: 240 | (240 << 8),
                normal: [0, 0, 0, 0],
                glint_uv: [0.0, 0.0],
            });
        }
    }
    for vertex in &mut out {
        vertex.layer &= !horizon_client::gfx::OIT_ENTITY;
    }
    ::core::ops::ControlFlow::Break((out, Vec::new()))
}
