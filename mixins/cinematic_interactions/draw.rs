// The crafting screen, in the scene: no window, only the inventory panel at the top, the recipe
// book button or panel at the left, the control hints at the bottom and the tooltips (a cell
// says what lies on it, or which ingredient the guide wants there). A furnace's screen is its
// front: the panel, its hint and what the zone under the cursor holds or takes.
fn cinematic_draw(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) -> ::core::ops::ControlFlow<()> {
    // FurnaceStation.tooltip and renderScreen: the panel at the top, the furnace's hint at the bottom
    // and, over a zone, its name in gold, then what it holds (or that it is empty), how far the
    // smelting is, how long the fire burns; with a stack in the hand, whether the zone takes it.
    // The station beside this one under the cursor: its outline and "Name  RMB — move over".
    fn draw_neighbour(game: &mut crate::game::Game<'_>, batch: &mut HudBatch, busy: bool) {
        use horizon_client_lib::native_cinematic_interactions_scene as ci;
        use horizon_client::hud::rgba;
        let (x, y) = game.cursor;
        let (t, yaw) = ci::table();
        let mut found: Option<([i32; 3], String, Vec<(f32, f32)>)> = None;
        if !busy && ci::phase() == ci::Phase::Inside && !ci::on_panel(batch.width, x, y) {
            for n in ci::neighbours(t, yaw) {
                let name = game.block_name(n).to_string();
                let Some(kind) = ci::kind_of(&name, false) else {
                    continue;
                };
                if !ci::moves_to(kind) {
                    continue;
                }
                let pts: Vec<(f32, f32)> = ci::block_corners(n).iter().filter_map(|c| ci::screen_of(*c, batch.width, batch.height)).collect();
                if pts.len() < 8 {
                    continue;
                }
                let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
                let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
                if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                    found = Some((n, name, pts));
                    break;
                }
            }
        }
        ci::set_neighbour(found.as_ref().map(|f| f.0));
        let Some((_, name, pts)) = found else {
            return;
        };
        let atlas = &game.gui.atlas;
        for (a, b) in ci::BLOCK_EDGES {
            ci::line(batch, atlas, pts[a], pts[b], 0.75, rgba(255, 255, 255, 200));
        }
        let title = game.translate(&format!("block.{}", name.replace(':', ".")), &name);
        let label = format!("{title}  \u{a7}7{}", ci::MOVE_OVER);
        let w = HudBatch::text_width(atlas, &label);
        let top = pts.iter().fold((0.0f32, f32::MAX), |(sx, sy), p| (sx + p.0 / 8.0, sy.min(p.1)));
        let (lx, ly) = ((top.0 - w * 0.5).clamp(2.0, batch.width - w - 2.0), (top.1 - 14.0).max(2.0));
        batch.fill(atlas, lx - 3.0, ly - 2.0, w + 6.0, 12.0, rgba(16, 12, 20, 200));
        batch.text(atlas, &label, lx, ly, rgba(255, 255, 255, 255), true);
    }
    fn draw_furnace(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) {
        use horizon_client_lib::native_cinematic_interactions_scene as ci;
        use horizon_client::text::Text;
        let kind = ci::kind();
        let (x, y) = game.cursor;
        let expanded = ci::update_expanded(batch.width, x, y);
        let hovered = game.slot_at(batch, x, y).filter(|s| *s < 36);
        ci::draw_panel(batch, &game.gui.atlas, expanded, game.selected, hovered);
        for i in 0..if expanded { 36 } else { 9 } {
            if let Some(stack) = game.inventory.get(i).cloned().flatten() {
                let (sx, sy) = ci::slot_pos(batch.width, i);
                game.draw_item(batch, &stack, sx, sy);
            }
        }
        if ci::config().hints {
            ci::draw_hint_line(batch, &game.gui.atlas, ci::station_hint(kind));
        }
        let busy = ci::furnace_hover(ci::ndc((x, y), batch.width, batch.height)).is_some();
        draw_neighbour(game, batch, busy);
        if ci::on_panel(batch.width, x, y) {
            game.draw_carried(batch);
            game.draw_slot_tooltip(batch);
            return;
        }
        let Some(slot) = ci::furnace_hover(ci::ndc((x, y), batch.width, batch.height)) else {
            return;
        };
        let state = game.container.and_then(|k| match game.world.block_entities.get(&k) {
            Some(horizon_client::containers::BlockEntity::Furnace(f)) => Some((f.lit_remaining, f.cooking_timer, f.cooking_total)),
            _ => None,
        });
        let (lit, cooked, total) = state.unwrap_or((0, 0, 0));
        let mut lines: Vec<(Text, u32)> = vec![(Text::literal(format!("\u{a7}6{}", ci::zone_title(kind, slot))), u32::MAX)];
        if let Some(carried) = game.carried.clone() {
            let fuel = &*carried.item == "minecraft:bucket" || game.items().get(&carried.item).is_some_and(|i| i.fuel.is_some());
            let cookable = game.recipes().cooking.iter().any(|c| c.kind == ci::cook_type(kind) && c.ingredient.test(&carried));
            if slot == 1 && !fuel {
                lines.push((Text::literal(format!("\u{a7}c{}", ci::NOT_FUEL)), u32::MAX));
            } else if slot == 2 {
                lines.push((Text::literal(format!("\u{a7}c{}", ci::NO_ROOM)), u32::MAX));
            } else if slot == 0 && !cookable {
                lines.push((Text::literal(format!("\u{a7}e{}", ci::not_cookable(kind))), u32::MAX));
            }
        } else {
            let held = game.slot_view(300 + slot);
            match held.as_ref() {
                None => lines.push((Text::literal(format!("\u{a7}7{}", ci::zone_empty(kind, slot))), u32::MAX)),
                Some(stack) => {
                    let name = horizon_client::item_tooltip::name_line(stack, &|k| game.pack.names.display_get(k));
                    lines.push((Text::literal(format!("{name} \u{d7}{}", stack.count)), u32::MAX));
                }
            }
            if slot == 0 && cooked > 0 && total > 0 {
                let percent = (f64::from(cooked) / f64::from(total) * 100.0).round() as u32;
                lines.push((Text::literal(format!("\u{a7}7{}", ci::progress_text(kind, percent))), u32::MAX));
            } else if slot == 1 && lit > 0 {
                lines.push((Text::literal(format!("\u{a7}7{}", ci::burning_text(lit))), u32::MAX));
            } else if slot == 2 && held.is_some() {
                lines.push((Text::literal(format!("\u{a7}7{}", ci::FURNACE_TAKE)), u32::MAX));
            }
        }
        game.draw_tooltip_lines(batch, &lines, x, y);
    }
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    // SmithingStation and CartographyStation in the scene: the panel, the hint and, over a spot,
    // what goes there (gold), what lies there and, on the worked spot, what it makes; a stack
    // the spot can't take is refused in red.
    // The crafter in the scene: the panel, its hint and, over a window, what lies in it (and that a
    // click takes it), that it is shut (and that an empty hand opens it), or that an empty hand
    // shuts it.
    fn draw_crafter(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) {
        use horizon_client_lib::native_cinematic_interactions_scene as ci;
        use horizon_client::text::Text;
        let (x, y) = game.cursor;
        let expanded = ci::update_expanded(batch.width, x, y);
        let hovered = game.slot_at(batch, x, y).filter(|s| *s < 36);
        ci::draw_panel(batch, &game.gui.atlas, expanded, game.selected, hovered);
        for i in 0..if expanded { 36 } else { 9 } {
            if let Some(stack) = game.inventory.get(i).cloned().flatten() {
                let (sx, sy) = ci::slot_pos(batch.width, i);
                game.draw_item(batch, &stack, sx, sy);
            }
        }
        if ci::config().hints {
            ci::draw_hint_line(batch, &game.gui.atlas, ci::station_hint(ci::Kind::Crafter));
        }
        let seconds = game.started.elapsed().as_secs_f64();
        let on_panel = ci::on_panel(batch.width, x, y);
        let cell = if on_panel { None } else { ci::hover(ci::ndc((x, y), batch.width, batch.height), seconds) };
        draw_neighbour(game, batch, matches!(cell, Some(ci::Hover::Cell(_))));
        if on_panel {
            game.draw_carried(batch);
            game.draw_slot_tooltip(batch);
            return;
        }
        let Some(ci::Hover::Cell(i)) = cell else {
            return;
        };
        let t = ci::table().0;
        let shut = match game.world.block_entities.get(&(t[0], t[1], t[2])) {
            Some(horizon_client::containers::BlockEntity::Crafter(c)) => c.disabled[i],
            _ => false,
        };
        let held = game.slot_view(300 + i);
        let mut lines: Vec<(Text, u32)> = Vec::new();
        if shut {
            lines.push((Text::literal(format!("\u{a7}c{}", ci::CRAFTER_SHUT)), u32::MAX));
            lines.push((Text::literal(format!("\u{a7}7{}", ci::CRAFTER_OPEN_IT)), u32::MAX));
        } else if let Some(stack) = held.as_ref() {
            if game.carried.is_none() {
                lines.extend(game.stack_tooltip(stack));
                lines.push((Text::literal(format!("\u{a7}7{}", ci::SURFACE_TAKE)), u32::MAX));
            }
        } else if game.carried.is_none() {
            lines.push((Text::literal(format!("\u{a7}7{}", ci::CRAFTER_SHUT_IT)), u32::MAX));
        }
        if !lines.is_empty() {
            game.draw_tooltip_lines(batch, &lines, x, y);
        }
    }
    fn draw_surface(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) {
        use horizon_client_lib::native_cinematic_interactions_scene as ci;
        use horizon_client::text::Text;
        use horizon_client::hud::rgba;
        let kind = ci::kind();
        let (x, y) = game.cursor;
        let expanded = ci::update_expanded(batch.width, x, y);
        let hovered = game.slot_at(batch, x, y).filter(|s| *s < 36);
        ci::draw_panel(batch, &game.gui.atlas, expanded, game.selected, hovered);
        for i in 0..if expanded { 36 } else { 9 } {
            if let Some(stack) = game.inventory.get(i).cloned().flatten() {
                let (sx, sy) = ci::slot_pos(batch.width, i);
                game.draw_item(batch, &stack, sx, sy);
            }
        }
        if ci::config().hints {
            ci::draw_hint_line(batch, &game.gui.atlas, ci::station_hint(kind));
        }
        // The enchanting table's runes and the beacon's powers, floating over the block.
        let (t, yaw) = ci::table();
        let mut extra: Option<Vec<(Text, u32)>> = None;
        let over = !ci::on_panel(batch.width, x, y);
        if kind == ci::Kind::Enchanting
            && let Some(st) = game.station.as_ref()
        {
            let offers = st.offers.clone();
            let lapis = st.inputs.get(1).cloned().flatten().map_or(0, |l| l.count as i32);
            let has_item = st.inputs.first().cloned().flatten().is_some();
            let level = game.experience.level;
            let creative = game.creative;
            let seed = game.level.xp_seed;
            let seconds = game.started.elapsed().as_secs_f32();
            let atlas = &game.gui.atlas;
            // The lapis socket: a blue circle on the table, faint when empty, with the count above.
            let corners = ci::lapis_socket(t, yaw);
            let screen: Vec<Option<(f32, f32)>> = corners.iter().map(|c| ci::screen_of(*c, batch.width, batch.height)).collect();
            if screen.iter().all(|c| c.is_some()) {
                let c = [screen[0].unwrap(), screen[1].unwrap(), screen[2].unwrap(), screen[3].unwrap()];
                ci::pixel_circle(batch, atlas, [[c[0].0, c[0].1], [c[1].0, c[1].1], [c[2].0, c[2].1], [c[3].0, c[3].1]], [96, 156, 255], if lapis > 0 { 0.95 } else { 0.5 });
            }
            if lapis > 1
                && let Some((sx, sy)) = ci::screen_of(ci::spot_point(t, yaw, &ci::ENCHANTING_SPOTS[1]) + glam::DVec3::Y * 0.1, batch.width, batch.height)
            {
                let label = format!("x{lapis}");
                let w = HudBatch::text_width(atlas, &label);
                batch.text(atlas, &label, sx - w * 0.5, sy - 16.0, rgba(150, 175, 225, 255), false);
            }
            let hovered = if over { ci::rune_at(x, y, batch.width, batch.height) } else { None };
            for (i, p) in ci::rune_points(t, yaw).iter().enumerate() {
                let cost = offers.costs[i];
                let Some((sx, sy)) = ci::screen_of(*p, batch.width, batch.height) else {
                    continue;
                };
                if cost <= 0 || !has_item {
                    continue;
                }
                let can = creative || (lapis > i as i32 && level >= cost);
                let r = ci::CIRCLE_RADIUS * if hovered == Some(i) { 1.12 } else { 1.0 };
                let turn = seconds * if i == 1 { -0.35 } else { 0.35 };
                ci::magic_circle(batch, atlas, sx, sy, r, turn, if can { [200, 84, 255] } else { [150, 140, 170] }, if can { 1.0 } else { 0.55 });
                let text = cost.to_string();
                let w = HudBatch::text_width(atlas, &text);
                let start = batch.vertices.len();
                batch.text(atlas, &text, sx - w * 0.5, sy - 4.0, if can { rgba(255, 255, 255, 255) } else { rgba(150, 150, 160, 255) }, true);
                batch.scale_since(start, sx, sy, 2.0, 2.0);
                let clue = horizon_client::text::Text::styled(
                    ci::clue_words(seed, i),
                    horizon_client::text::Style { font: Some("minecraft:alt".into()), color: Some(if can { 0xFFD8CCFF } else { 0xFF8F8A99 }), ..Default::default() },
                );
                let cw = atlas.font.width_text(&clue);
                batch.text_styled(atlas, &clue, sx - cw * 0.5, sy + r + 3.0, if can { rgba(216, 204, 255, 255) } else { rgba(143, 138, 153, 255) }, true);
                for d in 0..=i {
                    ci::star(batch, atlas, sx + (d as f32 - i as f32 * 0.5) * 7.0, sy + r + 17.0, if lapis > i as i32 { [96, 150, 255] } else { [90, 90, 120] });
                }
                if hovered == Some(i) {
                    let mut lines: Vec<(Text, u32)> = Vec::new();
                    if let Some((id, l)) = &offers.clues[i] {
                        let (name, _) = game.stations().enchantments.full_name(id, *l);
                        let clue = game.translate("container.enchant.clue", "%s . . . ?").replace("%s", &name);
                        lines.push((Text::literal(format!("\u{a7}f{clue}")), u32::MAX));
                    }
                    if !creative {
                        if level < cost {
                            lines.push((Text::literal(game.translate("container.enchant.level.requirement", "Level Requirement: %s").replace("%s", &cost.to_string())), rgba(255, 85, 85, 255)));
                        } else {
                            let n = i + 1;
                            let l = if n == 1 { game.translate("container.enchant.lapis.one", "1 Lapis Lazuli") } else { game.translate("container.enchant.lapis.many", "%s Lapis Lazuli").replace("%s", &n.to_string()) };
                            lines.push((Text::literal(l), if lapis >= n as i32 { rgba(170, 170, 170, 255) } else { rgba(255, 85, 85, 255) }));
                            let l = if n == 1 { game.translate("container.enchant.level.one", "1 Enchantment Level") } else { game.translate("container.enchant.level.many", "%s Enchantment Levels").replace("%s", &n.to_string()) };
                            lines.push((Text::literal(l), rgba(170, 170, 170, 255)));
                        }
                    }
                    lines.push((Text::literal("\u{a7}7Touch the rune to enchant"), u32::MAX));
                    extra = Some(lines);
                }
            }
        }
        // The anvil: the name of what lies on it floats above, with the cost of the work in green.
        // Over the name a pencil shows and a click opens the "New name" box low in the middle of the
        // screen (typing goes to it until Enter or Esc).
        ci::set_name_rect(None);
        if kind == ci::Kind::Anvil
            && let Some(st) = game.station.as_ref()
        {
            let shown = st.result.clone().or_else(|| st.inputs.first().cloned().flatten());
            let cost = st.anvil.cost;
            let renamed = st.result.is_some();
            let field = (st.rename.value.clone(), st.rename.cursor, st.rename.highlight, st.rename.cursor_visible());
            if let Some(stack) = shown
                && let Some((sx, sy)) = ci::screen_of(ci::spot_point(t, yaw, &ci::ANVIL_SPOTS[0]) + glam::DVec3::Y * 0.2, batch.width, batch.height)
            {
                let atlas = &game.gui.atlas;
                let plain = ci::plain_name(&horizon_client::item_tooltip::name_line(&stack, &|k| game.pack.names.display_get(k)));
                let bare = HudBatch::text_width(atlas, &plain);
                let rect = [sx - bare * 0.7 - 12.0, sy - 8.4, bare * 1.4 + 12.0, 11.2];
                let on = over && !ci::renaming() && x >= rect[0] && x <= rect[0] + rect[2] && y >= rect[1] && y <= rect[1] + rect[3];
                ci::set_name_rect(Some(rect));
                let label = if on { format!("\u{a7}e\u{270e} \u{a7}f{plain}") } else { plain.clone() };
                let w = HudBatch::text_width(atlas, &label);
                let color = if on || !renamed { rgba(255, 255, 255, 255) } else { rgba(255, 221, 102, 255) };
                let start = batch.vertices.len();
                batch.text(atlas, &label, sx - w * 0.5, sy - 6.0, color, true);
                batch.scale_since(start, sx, sy, 1.4, 1.4);
                if cost > 0 && renamed {
                    let text = cost.to_string();
                    let cw = HudBatch::text_width(atlas, &text);
                    let start = batch.vertices.len();
                    batch.text(atlas, &text, sx - cw * 0.5, sy - 26.0, if game.creative || game.experience.level >= cost { rgba(128, 255, 32, 255) } else { rgba(255, 85, 85, 255) }, true);
                    batch.scale_since(start, sx, sy - 22.0, 2.0, 2.0);
                }
                if on {
                    extra = Some(vec![(Text::literal(format!("\u{a7}7{}", ci::NAME_CLICK)), u32::MAX)]);
                }
            }
            // The rename box: "New name" over the field, the text selected when it opens.
            if ci::renaming() {
                let atlas = &game.gui.atlas;
                let [bx, by, bw, bh] = ci::name_box(batch.width, batch.height);
                batch.fill(atlas, bx, by, bw, bh, rgba(0, 0, 0, 128));
                batch.fill(atlas, bx, by + bh * 0.55, bw, bh * 0.45, rgba(0, 0, 0, 40));
                batch.fill(atlas, bx, by + bh - 1.0, bw, 1.0, rgba(196, 160, 72, 255));
                ci::scaled_text(batch, atlas, ci::NEW_NAME, bx + 3.0, by + 2.5, 0.7, rgba(150, 150, 150, 255));
                let (value, cursor, highlight, blink) = field;
                let chars: Vec<char> = value.chars().collect();
                let (a, b) = (cursor.min(highlight).min(chars.len()), cursor.max(highlight).min(chars.len()));
                let (tx, ty, k) = (bx + 3.0, by + 11.5, 0.7);
                let before: String = chars[..a].iter().collect();
                let picked: String = chars[a..b].iter().collect();
                let after: String = chars[b..].iter().collect();
                let wa = HudBatch::text_width(atlas, &before) * k;
                let wp = HudBatch::text_width(atlas, &picked) * k;
                ci::scaled_text(batch, atlas, &before, tx, ty, k, rgba(255, 255, 255, 255));
                if !picked.is_empty() {
                    batch.fill(atlas, tx + wa, ty - 0.5, wp, 7.0, rgba(255, 255, 255, 255));
                    ci::scaled_text(batch, atlas, &picked, tx + wa, ty, k, rgba(0, 0, 255, 255));
                }
                ci::scaled_text(batch, atlas, &after, tx + wa + wp, ty, k, rgba(255, 255, 255, 255));
                if picked.is_empty() && blink {
                    let wc = HudBatch::text_width(atlas, &chars[..cursor.min(chars.len())].iter().collect::<String>()) * k;
                    ci::scaled_text(batch, atlas, "_", tx + wc, ty, k, rgba(255, 255, 255, 255));
                }
            }
        }
        if kind == ci::Kind::Beacon
            && let Some(st) = game.station.as_ref()
        {
            let sel = st.beacon;
            let pos = st.pos;
            let levels = st.beacon_levels.unwrap_or_else(|| match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
                Some(horizon_client::containers::BlockEntity::Beacon(b)) => b.levels,
                _ => 0,
            });
            let index = |e: &str| ["speed", "haste", "resistance", "jump_boost", "strength", "regeneration"].iter().position(|n| *n == e);
            for (q, a) in ci::beacon_socket(pos) {
                if let Some(sq) = ci::screen_quad(&q, batch.width, batch.height) {
                    ci::fill_screen_quad(batch, &game.gui.atlas, sq, [235, 250, 255], a * 0.8);
                }
            }
            let hovered = if over { ci::power_at(x, y, batch.width, batch.height) } else { None };
            for (k, p) in ci::POWERS.iter().enumerate() {
                // The upgrade shows the chosen primary, at level II.
                let effect = if p.upgrade { match sel[0] { Some(e) => ["speed", "haste", "resistance", "jump_boost", "strength", "regeneration"][e], None => continue } } else { p.effect };
                let e = index(effect);
                let Some((sx, sy)) = ci::screen_of(ci::power_point(t, yaw, p), batch.width, batch.height) else {
                    continue;
                };
                let active = if p.upgrade || !p.primary { levels >= 4 } else { p.level <= levels };
                let chosen = if p.primary { sel[0] == e } else if p.upgrade { sel[1] == sel[0] && sel[0].is_some() } else { sel[1] == e && sel[1] != sel[0] };
                // BeaconStation.drawPower: a dim glow, the effect's ring (brighter when chosen or
                // hovered), the effect's sprite, and the pyramid level under it.
                let r = ci::CIRCLE_RADIUS * if hovered == Some(k) { 1.08 } else { 1.0 };
                let rgb = ci::effect_rgb(effect);
                let lit = if chosen { 1.0 } else if hovered == Some(k) { 0.85 } else if active { 0.6 } else { 0.35 };
                ci::glow(batch, &game.gui.atlas, sx, sy, r * 1.1, [18, 20, 24], 0.35);
                ci::ring(batch, &game.gui.atlas, sx, sy, r, if active || chosen { rgb } else { [150, 150, 160] }, lit);
                let tint = if active { u32::MAX } else { rgba(90, 90, 96, 255) };
                batch.sprite_scaled(&game.gui.atlas, &format!("mob_effect/{effect}"), sx - 9.0, sy - 9.0, 18.0, 18.0, tint);
                let label = if p.upgrade { "II" } else { ci::roman(p.level as u32) };
                let lw = HudBatch::text_width(&game.gui.atlas, label);
                batch.text(&game.gui.atlas, label, sx - lw * 0.5, sy + r + 2.0, if chosen { rgba(0xFF, 0xF2, 0xC0, 255) } else { rgba(0x9A, 0x9A, 0xA6, 255) }, false);
                if hovered == Some(k) {
                    let name = game.translate(&format!("effect.minecraft.{effect}"), effect);
                    let mut lines = vec![(Text::literal(format!("\u{a7}f{name}{}", if p.upgrade { " II" } else { "" })), u32::MAX)];
                    lines.push((Text::literal(if p.primary { "\u{a7}7Primary Power" } else { "\u{a7}7Secondary Power" }), u32::MAX));
                    let need = if p.primary { p.level } else { 4 };
                    let status = if levels <= 0 {
                        "\u{a7}cThe beacon stands on no pyramid".to_owned()
                    } else if !active {
                        format!("\u{a7}cNeeds a pyramid of level {need}")
                    } else if chosen {
                        "\u{a7}aChosen".to_owned()
                    } else if p.upgrade && sel[0].is_none() {
                        "\u{a7}7Choose a primary power first".to_owned()
                    } else {
                        "\u{a7}7Click \u{2014} choose".to_owned()
                    };
                    lines.push((Text::literal(status), u32::MAX));
                    extra = Some(lines);
                }
            }
        }
        // The loom's board: the game's own pattern buttons, at the right, once a banner and a dye lie
        // there (LoomStation.renderBoard).
        if kind == ci::Kind::Loom
            && let Some(st) = game.station.as_ref()
        {
            let (ox, oy) = ci::loom_board(batch.width, batch.height);
            let display = st.loom.display;
            let banner = st.inputs.first().is_some_and(|i| i.is_some());
            let full = banner && !display && st.inputs.get(1).is_some_and(|i| i.is_some());
            if display || banner {
                let atlas = &game.gui.atlas;
                batch.fill(atlas, ox + 54.0, oy - 6.0, 124.0, 80.0, rgba(16, 12, 8, 190));
                let title = "Patterns";
                let tw = HudBatch::text_width(atlas, title);
                batch.text(atlas, title, ox + 116.0 - tw * 0.5, oy - 4.0, rgba(236, 226, 196, 255), true);
            }
            if display {
                let tip = game.draw_loom(batch, ox, oy, (x - ox, y - oy));
                if over && ci::on_loom_board(batch.width, batch.height, x, y) {
                    // LoomStation: the pattern's name, then what a click does (gold).
                    extra = tip.map(|mut lines| {
                        lines.truncate(1);
                        lines.push((Text::literal(format!("\u{a7}6{}", ci::surface_work(kind))), u32::MAX));
                        lines
                    });
                }
            } else if banner {
                let atlas = &game.gui.atlas;
                let text = if full { "The banner has six patterns \u{2014} no room for more" } else { "Lay a dye and the patterns appear" };
                ci::scaled_text(batch, atlas, text, ox + 58.0, oy + 30.0, 0.5, rgba(200, 190, 170, 255));
            }
        }
        // The rituals' light: the enchanting over the item, the beacon over its core, the anvil's
        // sparks where the hammer hits.
        if let Some(age) = ci::burst_age(kind) {
            let atlas = &game.gui.atlas;
            match kind {
                ci::Kind::Enchanting => {
                    if let Some((sx, sy)) = ci::screen_of(ci::spot_point(t, yaw, &ci::ENCHANTING_SPOTS[0]), batch.width, batch.height) {
                        ci::draw_burst(batch, atlas, sx, sy, age, [214, 120, 255], true);
                    }
                }
                ci::Kind::Beacon => {
                    let primary = game.station.as_ref().and_then(|s| s.beacon[0]);
                    let rgb = primary.map_or([230, 250, 255], |e| ci::effect_rgb(["speed", "haste", "resistance", "jump_boost", "strength", "regeneration"][e]));
                    if let Some((sx, sy)) = ci::screen_of(ci::top_center(t) - glam::DVec3::Y * 0.12, batch.width, batch.height) {
                        ci::draw_burst(batch, atlas, sx, sy, age, rgb, true);
                    }
                }
                ci::Kind::Anvil | ci::Kind::Grindstone => {
                    let spot = if kind == ci::Kind::Anvil { &ci::ANVIL_SPOTS[0] } else { &ci::GRINDSTONE_SPOTS[0] };
                    if let Some(a) = ci::forge_spark_age()
                        && let Some((sx, sy)) = ci::screen_of(ci::spot_point(t, yaw, spot), batch.width, batch.height)
                    {
                        // The anvil: orange sparks at each strike; the wheel: orange and pink.
                        let rgb = if kind == ci::Kind::Anvil || (a * 10.0) as u32 % 2 == 0 { [255, 170, 60] } else { [255, 150, 230] };
                        ci::draw_burst(batch, atlas, sx, sy, a * 2.2, rgb, false);
                    }
                }
                _ => {}
            }
        }
        // The stonecutter: under each shape how many a cut gives ("\u{d7}2"); over a shape its name and
        // what a click does, or that the hand holds something else.
        if kind == ci::Kind::Stonecutter
            && let Some(st) = game.station.as_ref()
            && st.inputs.first().is_some_and(|i| i.is_some())
        {
            let all: Vec<ItemStack> = st.stone.recipes.iter().filter_map(|r| game.stations().recipes.stonecutting.get(*r).map(|c| c.result.clone())).collect();
            let range = ci::stone_shown(all.len());
            let shapes = all[range].to_vec();
            let seconds = game.started.elapsed().as_secs_f64();
            let hovered = if over { ci::stone_at(ci::ndc((x, y), batch.width, batch.height), shapes.len(), seconds) } else { None };
            let atlas = &game.gui.atlas;
            for (k, stack) in shapes.iter().enumerate() {
                if stack.count <= 1 {
                    continue;
                }
                let p = ci::stone_point(t, yaw, k, shapes.len(), seconds);
                if let (Some((sx, _)), Some((_, ly))) = (ci::screen_of(p, batch.width, batch.height), ci::screen_of(p - glam::DVec3::Y * 0.11, batch.width, batch.height)) {
                    let label = format!("\u{d7}{}", stack.count);
                    let lw = HudBatch::text_width(atlas, &label);
                    batch.text(atlas, &label, sx - lw * 0.5 + 3.0, ly, rgba(255, 255, 255, 255), true);
                }
            }
            if let Some(k) = hovered {
                let stack = &shapes[k];
                let named = horizon_client::item_tooltip::name_line(stack, &|key| game.pack.names.display_get(key));
                let title = if stack.count > 1 { format!("{named} \u{d7}{}", stack.count) } else { named };
                let busy = game.carried.as_ref().is_some_and(|c| c.item != stack.item);
                extra = Some(vec![
                    (Text::literal(title), u32::MAX),
                    (Text::literal(if busy { format!("\u{a7}c{}", ci::STONE_BUSY) } else { format!("\u{a7}6{}", ci::STONE_CUT) }), u32::MAX),
                ]);
            }
        }
        let busy = extra.is_some() || ci::surface_hover(ci::ndc((x, y), batch.width, batch.height)).is_some();
        draw_neighbour(game, batch, busy);
        if let Some(lines) = extra {
            game.draw_tooltip_lines(batch, &lines, x, y);
            return;
        }
        if ci::on_panel(batch.width, x, y) {
            game.draw_carried(batch);
            game.draw_slot_tooltip(batch);
            return;
        }
        let Some(slot) = ci::surface_hover(ci::ndc((x, y), batch.width, batch.height)) else {
            return;
        };
        let name = |game: &crate::game::Game<'_>, stack: &ItemStack| horizon_client::item_tooltip::name_line(stack, &|k| game.pack.names.display_get(k));
        // The chiseled bookshelf: the book in the place, or what an empty one takes.
        if kind == ci::Kind::Shelf {
            let t = ci::table().0;
            let book = match game.world.block_entities.get(&(t[0], t[1], t[2])) {
                Some(horizon_client::containers::BlockEntity::ChiseledBookshelf(b)) => b.items.get(slot).cloned().flatten(),
                _ => None,
            };
            let state = game.world.block(t[0], t[1], t[2]);
            let key = format!("slot_{slot}_occupied");
            let full = game.pack.assets.registry.state(state).and_then(|s| s.properties.get(&key)).is_some_and(|v| v == "true");
            let mut lines: Vec<(Text, u32)> = Vec::new();
            if full {
                lines.push((Text::literal(book.as_ref().map_or_else(|| "Book".to_owned(), |b| name(game, b))), u32::MAX));
                lines.push((Text::literal("\u{a7}6LMB \u{2014} take"), u32::MAX));
                lines.push((Text::literal("\u{a7}7Shift+LMB \u{2014} into the inventory"), u32::MAX));
            } else if let Some(c) = game.carried.as_ref() {
                if ci::is_book(&c.item) {
                    lines.push((Text::literal(name(game, c)), u32::MAX));
                    lines.push((Text::literal("\u{a7}6LMB \u{2014} put it here"), u32::MAX));
                } else {
                    lines.push((Text::literal("\u{a7}cOnly books go here"), u32::MAX));
                }
            } else {
                lines.push((Text::literal("\u{a7}7An empty place: take a book from the panel at the top"), u32::MAX));
            }
            game.draw_tooltip_lines(batch, &lines, x, y);
            return;
        }
        // The campfire: the food on the spot and how far it is cooked; carried food goes on the
        // first free spot, what can't be cooked is refused.
        if kind == ci::Kind::Campfire {
            let t = ci::table().0;
            let fire = match game.world.block_entities.get(&(t[0], t[1], t[2])) {
                Some(horizon_client::containers::BlockEntity::Campfire(c)) => Some((**c).clone()),
                _ => None,
            };
            let mut lines: Vec<(Text, u32)> = Vec::new();
            let on = fire.as_ref().and_then(|c| c.items.get(slot).cloned().flatten());
            let full = fire.as_ref().is_some_and(|c| c.items.iter().all(|i| i.is_some()));
            if let Some(food) = game.carried.clone() {
                if game.recipes().cook("minecraft:campfire_cooking", &food).is_none() {
                    lines.push((Text::literal("\u{a7}cThis can't be cooked on a campfire"), u32::MAX));
                } else if full {
                    lines.push((Text::literal("\u{a7}cNo free spot on the fire"), u32::MAX));
                } else {
                    lines.push((Text::literal(format!("\u{a7}6{}", ci::plain_name(&name(game, &food)))), u32::MAX));
                    lines.push((Text::literal("\u{a7}7Click \u{2014} onto the fire"), u32::MAX));
                }
            } else if let (Some(food), Some(c)) = (on.as_ref(), fire.as_ref()) {
                lines.push((Text::literal(name(game, food)), u32::MAX));
                let (p, total) = (c.progress[slot], c.total[slot]);
                if total > 0 {
                    let pct = (p * 100 / total).clamp(0, 100);
                    let left = ((total - p).max(0) + 19) / 20;
                    lines.push((Text::literal(format!("\u{a7}7Cooking: {pct}% \u{b7} ~{left} s left")), u32::MAX));
                }
            }
            if !lines.is_empty() {
                game.draw_tooltip_lines(batch, &lines, x, y);
            }
            return;
        }
        let held = game.slot_view(ci::spot_base(kind) + slot);
        let result = game.slot_view(650);
        let mut lines: Vec<(Text, u32)> = Vec::new();
        let counted = |game: &crate::game::Game<'_>, stack: &ItemStack| {
            let n = name(game, stack);
            if stack.count > 1 { format!("{n} \u{d7}{}", stack.count) } else { n }
        };
        if let Some(c) = game.carried.clone() {
            if held.is_none() && !game.quick_craft_accepts(ci::spot_base(kind) + slot, 0, 0) {
                lines.push((Text::literal(format!("\u{a7}c{}", ci::surface_refuse(kind))), u32::MAX));
            } else {
                // What the hand would lay there.
                lines.extend(game.stack_tooltip(&c));
            }
        } else if let Some(stack) = held.as_ref() {
            if kind == ci::Kind::Beacon {
                lines.push((Text::literal(format!("\u{a7}6{}", ci::spot_title(kind, slot))), u32::MAX));
                lines.push((Text::literal(counted(game, stack)), u32::MAX));
                let chosen = game.station.as_ref().is_some_and(|s| s.beacon[0].is_some());
                lines.push((Text::literal(if chosen { "\u{a7}aClick \u{2014} light the beacon" } else { "\u{a7}eChoose a power over the beacon first" }), u32::MAX));
                lines.push((Text::literal("\u{a7}7Right-click \u{2014} take it"), u32::MAX));
            } else if kind == ci::Kind::Enchanting && slot == 1 {
                // The lapis in its rune: how much lies there.
                lines.push((Text::literal(counted(game, stack)), u32::MAX));
            } else if kind == ci::Kind::Stonecutter {
                lines.push((Text::literal(counted(game, stack)), u32::MAX));
                lines.push((Text::literal(format!("\u{a7}7{}", ci::SURFACE_TAKE)), u32::MAX));
            } else {
                // The item's own tooltip, then the station's lines.
                lines.extend(game.stack_tooltip(stack));
                match kind {
                    ci::Kind::Enchanting => {
                        lines.push((Text::literal(format!("\u{a7}7{}", ci::TAKE_BACK)), u32::MAX));
                    }
                    ci::Kind::Brewing => {
                        if slot == 4 {
                            let t = ci::table().0;
                            let fuel = match game.world.block_entities.get(&(t[0], t[1], t[2])) {
                                Some(horizon_client::containers::BlockEntity::Brewing(b)) => b.fuel,
                                _ => 0,
                            };
                            lines.push((Text::literal(format!("\u{a7}6{}", ci::brewing_fuel_text(fuel))), u32::MAX));
                        }
                        lines.push((Text::literal(format!("\u{a7}7{}", ci::SURFACE_TAKE)), u32::MAX));
                    }
                    _ => {
                        if ci::tooltip_gap(kind) {
                            lines.push((Text::literal(""), u32::MAX));
                        }
                        match (result.as_ref(), kind) {
                            (Some(made), ci::Kind::Smithing) => {
                                lines.push((Text::literal(format!("\u{a7}7Makes: {}", ci::plain_name(&name(game, made)))), u32::MAX));
                                lines.push((Text::literal(format!("\u{a7}6{}", ci::surface_work(kind))), u32::MAX));
                            }
                            (Some(_), ci::Kind::Anvil) => {
                                let cost = game.station.as_ref().map_or(0, |s| s.anvil.cost);
                                if cost > 0 {
                                    lines.push((Text::literal(format!("\u{a7}aEnchantment Cost: {cost}")), u32::MAX));
                                }
                                lines.push((Text::literal(format!("\u{a7}6{}", ci::surface_work(kind))), u32::MAX));
                            }
                            (Some(_), ci::Kind::Cartography) => {
                                let addition = game.slot_view(601).map(|s| s.item.to_string()).unwrap_or_default();
                                lines.push((Text::literal(format!("\u{a7}7{}", ci::cartography_effect(&addition))), u32::MAX));
                                lines.push((Text::literal(format!("\u{a7}6{}", ci::surface_work(kind))), u32::MAX));
                            }
                            (Some(_), _) => {
                                if let Some(effect) = ci::work_effect(kind) {
                                    lines.push((Text::literal(effect), u32::MAX));
                                }
                                lines.push((Text::literal(format!("\u{a7}6{}", ci::surface_work(kind))), u32::MAX));
                            }
                            (None, _) => {
                                // SmithingStation: what is still missing, then the take.
                                if kind == ci::Kind::Smithing {
                                    for other in [0usize, 1, 2] {
                                        if game.slot_view(600 + other).is_none() {
                                            lines.push((Text::literal(format!("\u{a7}8{}", ci::smithing_missing(other))), u32::MAX));
                                        }
                                    }
                                }
                                lines.push((Text::literal(format!("\u{a7}7{}", ci::SURFACE_TAKE)), u32::MAX));
                            }
                        }
                    }
                }
            }
        } else {
            lines.push((Text::literal(format!("\u{a7}7{}", ci::spot_title(kind, slot))), u32::MAX));
        }
        game.draw_tooltip_lines(batch, &lines, x, y);
    }
    if !ci::controls() {
        return ::core::ops::ControlFlow::Continue(());
    }
    // Still walking to the table: the screen comes once there.
    if ci::walking(std::time::Instant::now()) {
        return ::core::ops::ControlFlow::Break(());
    }
    if ci::is_furnace(ci::kind()) {
        draw_furnace(game, batch);
        return ::core::ops::ControlFlow::Break(());
    }
    if ci::is_surface(ci::kind()) {
        draw_surface(game, batch);
        return ::core::ops::ControlFlow::Break(());
    }
    if ci::is_crafter() {
        draw_crafter(game, batch);
        return ::core::ops::ControlFlow::Break(());
    }
    let (x, y) = game.cursor;
    let expanded = ci::update_expanded(batch.width, x, y);
    let hovered = game.slot_at(batch, x, y).filter(|s| *s < 36);
    ci::draw_panel(batch, &game.gui.atlas, expanded, game.selected, hovered);
    for i in 0..if expanded { 36 } else { 9 } {
        if let Some(stack) = game.inventory.get(i).cloned().flatten() {
            let (sx, sy) = ci::slot_pos(batch.width, i);
            game.draw_item(batch, &stack, sx, sy);
        }
    }
    if !ci::book_open() {
        let on_book = ci::on_recipe(batch.height, x, y);
        ci::draw_recipe_button(batch, &game.gui.atlas, on_book);
        let [bx, by, bw, bh] = ci::recipe_button(batch.height);
        let start = batch.vertices.len();
        game.draw_item(batch, &ItemStack::new("minecraft:book", 1), bx + (bw - 16.0) * 0.5, by + (bh - 16.0) * 0.5);
        batch.scale_since(start, bx + bw * 0.5, by + bh * 0.5, 1.2, 1.2);
    }
    // A drag over the cells shares the stack out; the crafted item has its own hint.
    let spread = game.inv_drag.as_ref().and_then(|(ty, slots)| {
        let cells = slots.iter().filter(|s| (100..109).contains(*s)).count();
        ci::spread_text(game.carried.as_ref().map_or(0, |c| c.count as u32), cells, *ty == 1)
    });
    if let Some(text) = spread {
        ci::draw_hint_line(batch, &game.gui.atlas, &text);
    } else if ci::config().hints {
        let on_result = ci::hover(ci::ndc((x, y), batch.width, batch.height), game.started.elapsed().as_secs_f64()) == Some(ci::Hover::Result);
        ci::draw_hint_line(batch, &game.gui.atlas, if on_result { ci::RESULT_HINT } else { ci::HINTS });
    }
    // The recipe panel (the book's hook draws it in the scene).
    game.draw_recipe_book(batch);
    if ci::on_panel(batch.width, x, y) {
        game.draw_carried(batch);
    }
    let seconds = game.started.elapsed().as_secs_f64();
    let over_book = ci::book_open() && ci::book_hit(x, y, ci::selected().is_some()).is_some();
    let busy = over_book || ci::book_open() || ci::hover(ci::ndc((x, y), batch.width, batch.height), seconds).is_some();
    draw_neighbour(game, batch, busy);
    match ci::hover(ci::ndc((x, y), batch.width, batch.height), seconds) {
        _ if over_book || ci::on_panel(batch.width, x, y) => {
            if !over_book {
                game.draw_slot_tooltip(batch);
            }
        }
        Some(ci::Hover::Cell(i)) if game.carried.is_none() => {
            let name = |game: &crate::game::Game<'_>, stack: &ItemStack| horizon_client::item_tooltip::name_line(stack, &|k| game.pack.names.display_get(k));
            let lines = match (game.craft_grid.get(i).cloned().flatten(), game.ghost_stack(100 + i)) {
                (Some(stack), _) => Some([
                    (horizon_client::text::Text::literal(name(game, &stack)), u32::MAX),
                    (horizon_client::text::Text::literal(format!("\u{a7}7On this cell: {}", stack.count)), u32::MAX),
                ]),
                (None, Some(ghost)) => {
                    let have = game.inventory.iter().flatten().any(|s| s.item == ghost.item);
                    Some([
                        (horizon_client::text::Text::literal(name(game, &ghost)), u32::MAX),
                        (horizon_client::text::Text::literal(if have { "\u{a7}7Place this ingredient here" } else { "\u{a7}cMissing this ingredient" }), u32::MAX),
                    ])
                }
                _ => None,
            };
            if let Some(lines) = lines {
                game.draw_tooltip_lines(batch, &lines, x, y);
            }
        }
        _ => game.draw_slot_tooltip(batch),
    }
    ::core::ops::ControlFlow::Break(())
}
