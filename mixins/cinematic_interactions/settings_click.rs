// A button of the mod's page in Options > Mods: its setting changes and is saved.
fn cinematic_settings_click(mod_id: &str, button: usize) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if mod_id != ci::ID {
        return;
    }
    if !ci::loaded() {
        ci::load(crate::game::Game::native_config_dir().ok());
    }
    ci::setting_click(button);
}
