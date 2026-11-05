// The wheel on the settings: the list scrolls.
fn cinematic_options_scroll(game: &mut crate::game::Game<'_>, amount: f32) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client_lib::native_cinematic_menu as menu;
    if !menu::is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let (w, h) = game.gui_size();
    menu::scroll(w.ceil(), h.ceil(), amount);
    ::core::ops::ControlFlow::Break(Ok(()))
}
