// TableRecipeBook, in the scene: the "Recipes" panel at the left with its search box, its own
// "Have materials" filter, every crafting recipe that fits the table (sorted by its result's name,
// 16 a page), the pages and the chosen recipe with "Lay out". Each frame it also takes the steps of
// laying a recipe out that the hands have reached.
fn cinematic_book_draw(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) -> ::core::ops::ControlFlow<()> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    use std::hash::{Hash, Hasher};
    if !ci::controls() || !ci::table_scene() {
        return ::core::ops::ControlFlow::Continue(());
    }
    // Laying out: at each cell the hand reaches, it takes back what is in the way, or picks the
    // stack up from the inventory, lays what the cell needs and puts the rest back.
    for step in ci::take_lay_steps(std::time::Instant::now()) {
        use horizon_client::menu::Input;
        let cell = 100 + step.cell;
        if step.take {
            game.menu_ops(&[(Some(cell), 0, Input::QuickMove)]);
            continue;
        }
        if game.carried.is_some() {
            continue;
        }
        let from = (0..36usize).find(|&j| game.inventory.get(j).cloned().flatten().is_some_and(|s| step.items.iter().any(|i| i.as_str() == &*s.item)));
        let Some(j) = from else {
            continue;
        };
        let have = game.inventory.get(j).cloned().flatten().map_or(0, |s| s.count as u32);
        let mut ops = vec![(Some(j), 0, Input::Pickup)];
        for _ in 0..step.count.max(1).min(have) {
            ops.push((Some(cell), 1, Input::Pickup));
        }
        ops.push((Some(j), 0, Input::Pickup));
        game.menu_ops(&ops);
    }
    let open = game.book_open(BookType::Crafting);
    ci::set_book_open(open);
    if !open || ci::walking(std::time::Instant::now()) {
        return ::core::ops::ControlFlow::Break(());
    }
    game.ensure_book();
    let (x, y) = game.cursor;
    ci::set_gui(batch.width, batch.height);
    let sc = ci::book_scale();
    let (search, focused) = game.recipe_ui.search.as_ref().map_or((String::new(), false), |s| (s.value.clone(), s.focused));
    // The list, rebuilt when the search, the filter, the inventory or the grid changes.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    search.hash(&mut h);
    ci::book_filter().hash(&mut h);
    for s in game.inventory.iter().chain(game.craft_grid.iter()) {
        s.as_ref().map(|s| (s.item.to_string(), s.count)).hash(&mut h);
    }
    let key = h.finish() | 1;
    if ci::book_list(key).is_none() {
        let stock = game.book_available(BookType::Crafting);
        let mut list: Vec<(String, String, usize, bool)> = Vec::new();
        if let Some(book) = game.recipe_ui.book.as_ref() {
            use horizon_client::recipe_book::Category;
            let mut seen = std::collections::HashSet::new();
            for (i, e) in book.entries.iter().enumerate() {
                let crafting = matches!(e.category, Category::CraftingBuilding | Category::CraftingRedstone | Category::CraftingEquipment | Category::CraftingMisc);
                if !crafting || !e.fits(3, 3) || e.result.count == 0 {
                    continue;
                }
                if book.placements.get(&e.id).is_none_or(|p| p.impossible()) || !seen.insert(e.id.clone()) {
                    continue;
                }
                let name = ci::plain_name(&horizon_client::item_tooltip::name_line(&e.result, &|k| game.pack.names.display_get(k)));
                if !ci::search_matches(&name, &search) {
                    continue;
                }
                let craftable = book.can_craft(e, &stock);
                if ci::book_filter() && !craftable {
                    continue;
                }
                list.push((name, e.id.clone(), i, craftable));
            }
        }
        ci::sort_recipes(&mut list);
        ci::set_book_list(key, list.into_iter().map(|(_, _, i, c)| (i, c)).collect());
    }
    let list = ci::book_list_current();
    let pages = list.len().div_ceil(ci::BOOK_PAGE).max(1);
    let page = ci::book_page().min(pages - 1);
    // The page's recipes: the result shown, whether it can be made, its entry.
    let mut shown: Vec<(ItemStack, bool, usize)> = Vec::new();
    if let Some(book) = game.recipe_ui.book.as_ref() {
        for &(i, craftable) in list.iter().skip(page * ci::BOOK_PAGE).take(ci::BOOK_PAGE) {
            if let Some(e) = book.entries.get(i) {
                shown.push((e.result.clone(), craftable, i));
            }
        }
    }
    let name = |game: &crate::game::Game<'_>, stack: &ItemStack| horizon_client::item_tooltip::name_line(stack, &|k| game.pack.names.display_get(k));
    let choice = ci::selected().and_then(|i| game.recipe_ui.book.as_ref()?.entries.get(i).map(|e| e.result.clone()));
    let choice_name = choice.as_ref().map(|s| name(game, s));
    let hover = ci::book_hit(x, y, choice.is_some());
    let blink = game.started.elapsed().as_millis() % 600 < 300;
    // Lay out: what the guide's cells want (any of the ghost's items), what lies in the grid and what
    // the inventory has.
    let wants: Vec<(usize, Vec<String>)> = game
        .recipe_ui
        .ghost
        .iter()
        .filter(|g| !g.result && (100..109).contains(&g.slot))
        .map(|g| (g.slot - 100, g.items.iter().map(|s| s.item.to_string()).collect()))
        .collect();
    let grid: Vec<Option<(String, u32)>> = (0..9).map(|c| game.craft_grid.get(c).cloned().flatten().map(|s| (s.item.to_string(), s.count as u32))).collect();
    let stock: Vec<(String, u32)> = game.inventory.iter().take(36).flatten().map(|s| (s.item.to_string(), s.count as u32)).collect();
    let shift = game.has_shift_down();
    let sets = ci::lay_sets(&wants, &stock);
    let plan = ci::lay_plan(&wants, &grid, &stock, if shift { sets } else { 1 });
    let fill = plan.iter().filter(|p| !p.take).count();
    let missing: Vec<String> = wants
        .iter()
        .filter(|(c, items)| grid[*c].is_none() && !stock.iter().any(|s| items.contains(&s.0)))
        .filter_map(|(c, _)| game.ghost_stack(100 + c))
        .map(|s| ci::plain_name(&name(game, &s)))
        .collect();
    let lay_text = if ci::laying() {
        "Laying out\u{2026}".to_owned()
    } else if choice.is_none() || plan.is_empty() {
        "Lay out".to_owned()
    } else if shift {
        if sets == 0 { "Lay out".to_owned() } else { format!("Lay out \u{d7}{sets}") }
    } else {
        format!("Lay out ({fill})")
    };
    let ready = !ci::laying() && !plan.is_empty() && (!shift || sets > 0);
    ci::draw_book(batch, &game.gui.atlas, &search, focused, ci::book_filter(), page, pages, choice_name.as_deref(), hover, blink, (&lay_text, ready));
    if list.is_empty() {
        let text = "No recipes found";
        let [bx, by, bw, _] = ci::book_rect();
        let w = HudBatch::text_width(&game.gui.atlas, text) * sc;
        ci::scaled_text(batch, &game.gui.atlas, text, bx + bw * 0.5 - w * 0.5, by + 110.0 * sc, sc, rgba(160, 160, 168, 255));
    }
    for (k, (stack, _, entry)) in shown.iter().enumerate() {
        let (ix, iy) = ci::book_icon(k);
        if ci::selected() == Some(*entry) {
            let gold = rgba(232, 204, 132, 230);
            let e = 2.0 * sc;
            let t = sc.max(1.0);
            batch.fill(&game.gui.atlas, ix - e, iy - e, 16.0 * sc + 2.0 * e, t, gold);
            batch.fill(&game.gui.atlas, ix - e, iy + 16.0 * sc + e - t, 16.0 * sc + 2.0 * e, t, gold);
            batch.fill(&game.gui.atlas, ix - e, iy - e, t, 16.0 * sc + 2.0 * e, gold);
            batch.fill(&game.gui.atlas, ix + 16.0 * sc + e - t, iy - e, t, 16.0 * sc + 2.0 * e, gold);
        }
        let start = batch.vertices.len();
        game.draw_item(batch, stack, ix, iy);
        batch.scale_since(start, ix, iy, sc, sc);
    }
    if let Some(ci::BookHit::Recipe(k)) = hover
        && let Some((stack, craftable, _)) = shown.get(k)
    {
        let lines = [
            (Text::literal(name(game, stack)), u32::MAX),
            (Text::literal(if *craftable { "\u{a7}aEnough materials for one craft" } else { "\u{a7}cMissing materials" }), u32::MAX),
            (Text::literal("\u{a7}7Click to show the tabletop guide"), u32::MAX),
        ];
        game.draw_tooltip_lines(batch, &lines, x, y);
    }
    if hover == Some(ci::BookHit::Clear) {
        game.draw_tooltip_lines(batch, &[(Text::literal("\u{a7}7\u{d7} \u{2014} clear the tabletop guide"), u32::MAX)], x, y);
    }
    if hover == Some(ci::BookHit::LayOut) {
        let mut lines = vec![if plan.is_empty() {
            "\u{a7}7Nothing to lay out: it is all there, or the inventory has none of it".to_owned()
        } else if shift {
            if sets == 0 { "\u{a7}7Shift: not enough ingredients for even one full set".to_owned() } else { format!("\u{a7}7Shift + click \u{2014} lay out up to {sets} sets") }
        } else {
            "\u{a7}7Take the materials from the inventory and lay them out on the table for the recipe".to_owned()
        }];
        let taking = plan.iter().filter(|p| p.take).count();
        if taking > 0 {
            lines.push(format!("\u{a7}7First the hands take back what is in the way: {taking}"));
        }
        if !missing.is_empty() {
            lines.push(format!("\u{a7}cMissing: {} \u{2014} those cells stay see-through", missing.join(", ")));
        }
        let lines: Vec<(Text, u32)> = lines.into_iter().map(|l| (Text::literal(l), u32::MAX)).collect();
        game.draw_tooltip_lines(batch, &lines, x, y);
    }
    ::core::ops::ControlFlow::Break(())
}
