// The Options screen, while the Cinematic Interactions settings are open: they are drawn in its
// place, the stations' items on their buttons.
fn cinematic_options_draw(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) -> ::core::ops::ControlFlow<()> {
    use horizon_client_lib::native_cinematic_menu as menu;
    if !menu::is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let g: &crate::game::Game<'_> = &*game;
    menu::draw(&g.gui.atlas, batch, g.cursor, &mut |b, item, x, y| {
        g.draw_item(b, &horizon_client::items::ItemStack::new(item, 1), x, y);
    });
    ::core::ops::ControlFlow::Break(())
}
