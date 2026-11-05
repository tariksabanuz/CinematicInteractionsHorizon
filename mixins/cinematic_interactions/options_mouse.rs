// The mouse on the Options screen: "Cinematic Interaction Settings..." opens the settings, and
// while they are open every press is theirs.
fn cinematic_options_mouse(
    game: &mut crate::game::Game<'_>,
    pressed: bool,
    right: bool,
) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client_lib::native_cinematic_menu as menu;
    use horizon_client_render::native_cinematic_button as cb;
    let (x, y) = game.cursor;
    if !menu::is_open() {
        if pressed && !right && game.ui.options.page() == Page::Main && cb::button_at(x, y) {
            menu::open(crate::game::Game::native_config_dir().ok());
            return ::core::ops::ControlFlow::Break(game.handle_option_events(vec![Event::ButtonClick]));
        }
        return ::core::ops::ControlFlow::Continue(());
    }
    if right {
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    game.dragging = pressed;
    let (w, h) = game.gui_size();
    let out = match menu::mouse(&game.gui.atlas, w.ceil(), h.ceil(), x, y, pressed) {
        menu::Out::None => Ok(()),
        menu::Out::Click | menu::Out::Close => game.handle_option_events(vec![Event::ButtonClick]),
    };
    ::core::ops::ControlFlow::Break(out)
}
