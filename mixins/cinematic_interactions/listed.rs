// The mod's row in Options > Mods; its settings are on the row's page.
fn cinematic_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    mods.push(crate::game::native_hooks::NativeMod {
        id: ci::ID.into(),
        title: format!("{} {}", ci::NAME, ci::VERSION),
        description: ci::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
