// Once a frame: the settings are read at the start, and the mod follows its switch in Options > Mods.
fn cinematic_frame(game: &mut crate::game::Game<'_>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::loaded() {
        ci::load(crate::game::Game::native_config_dir().ok());
    }
    ci::set_active(game.native_on(ci::ID));
}
