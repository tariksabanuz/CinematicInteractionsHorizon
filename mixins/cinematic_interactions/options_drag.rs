// The pointer moving with the button down on the settings: a slider follows it.
fn cinematic_options_drag(game: &mut crate::game::Game<'_>) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client_lib::native_cinematic_menu as menu;
    if !menu::is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    menu::drag(game.cursor.0);
    ::core::ops::ControlFlow::Break(Ok(()))
}
