// The mod's page in Options > Mods: one button per setting, each showing its value.
fn cinematic_settings(mod_id: &str, buttons: &mut Vec<String>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if mod_id != ci::ID {
        return;
    }
    if !ci::loaded() {
        ci::load(crate::game::Game::native_config_dir().ok());
    }
    buttons.extend(ci::setting_labels());
}
