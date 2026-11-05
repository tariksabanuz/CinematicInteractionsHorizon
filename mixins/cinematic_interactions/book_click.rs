// A click in the scene's recipe panel: the book button opens it, "-" closes it; the search box,
// the filter, the pages, a recipe (its tabletop guide), "x" and "Lay out" (the hands lay it out,
// cell by cell, with the menu's own clicks, so any recipe goes down, known or not).
fn cinematic_book_click(game: &mut crate::game::Game<'_>, right: bool) -> ::core::ops::ControlFlow<bool> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::controls() || !ci::table_scene() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else {
        return ::core::ops::ControlFlow::Break(false);
    };
    let batch = HudBatch::new(size);
    ci::set_gui(batch.width, batch.height);
    let (x, y) = game.cursor;
    let book = BookType::Crafting;
    let click = |game: &mut crate::game::Game<'_>| game.audio.play("minecraft:ui.button.click", None, 0.25, 1.0);
    if !game.book_open(book) {
        if !right && ci::on_recipe(batch.height, x, y) {
            game.level.recipe_book.open[book.index()] = true;
            game.send_book_settings(book);
            ci::set_book_open(true);
            click(game);
            return ::core::ops::ControlFlow::Break(true);
        }
        return ::core::ops::ControlFlow::Break(false);
    }
    game.ensure_book();
    let hit = ci::book_hit(x, y, ci::selected().is_some());
    if let Some(search) = game.recipe_ui.search.as_mut() {
        search.focused = hit == Some(ci::BookHit::Search);
    }
    let Some(hit) = hit else {
        return ::core::ops::ControlFlow::Break(false);
    };
    if right {
        return ::core::ops::ControlFlow::Break(true);
    }
    let list = ci::book_list_current();
    let pages = list.len().div_ceil(ci::BOOK_PAGE).max(1);
    let page = ci::book_page().min(pages - 1);
    match hit {
        ci::BookHit::Minimize => {
            game.level.recipe_book.open[book.index()] = false;
            game.send_book_settings(book);
            ci::set_book_open(false);
            click(game);
        }
        ci::BookHit::Filter => {
            ci::set_book_filter(!ci::book_filter());
            ci::set_book_page(0);
            click(game);
        }
        ci::BookHit::Prev => {
            ci::set_book_page((page + pages - 1) % pages);
            click(game);
        }
        ci::BookHit::Next => {
            ci::set_book_page((page + 1) % pages);
            click(game);
        }
        ci::BookHit::Recipe(k) => {
            let pick = list.get(page * ci::BOOK_PAGE + k).map(|e| e.0);
            let entry = pick.and_then(|i| game.recipe_ui.book.as_ref()?.entries.get(i).cloned());
            if let (Some(i), Some(entry)) = (pick, entry) {
                ci::select(Some(i));
                game.recipe_ui.ghost.clear();
                game.fill_ghost(book, &entry);
                click(game);
            }
        }
        ci::BookHit::Clear => {
            ci::select(None);
            ci::cancel_lay();
            game.recipe_ui.ghost.clear();
            click(game);
        }
        ci::BookHit::LayOut => {
            if ci::selected().is_some() && !ci::laying() {
                // The hands' plan: what the guide's cells want, what lies there, what the
                // inventory has; with Shift as many whole sets as it covers.
                let wants: Vec<(usize, Vec<String>)> = game
                    .recipe_ui
                    .ghost
                    .iter()
                    .filter(|g| !g.result && (100..109).contains(&g.slot))
                    .map(|g| (g.slot - 100, g.items.iter().map(|s| s.item.to_string()).collect()))
                    .collect();
                let grid: Vec<Option<(String, u32)>> = (0..9).map(|c| game.craft_grid.get(c).cloned().flatten().map(|s| (s.item.to_string(), s.count as u32))).collect();
                let stock: Vec<(String, u32)> = game.inventory.iter().take(36).flatten().map(|s| (s.item.to_string(), s.count as u32)).collect();
                let sets = if game.has_shift_down() { ci::lay_sets(&wants, &stock).min(64) } else { 1 };
                let plan = ci::lay_plan(&wants, &grid, &stock, sets);
                if !plan.is_empty() {
                    ci::start_lay(plan, std::time::Instant::now());
                    click(game);
                }
            }
        }
        ci::BookHit::Search | ci::BookHit::Panel => {}
    }
    ::core::ops::ControlFlow::Break(true)
}
