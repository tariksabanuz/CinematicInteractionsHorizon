// hasClickedOutside, in the scene: there is no window to click out of, so nothing is thrown.
fn cinematic_outside(batch: &HudBatch) -> ::core::ops::ControlFlow<bool> {
    use horizon_client::native_cinematic_interactions as ci;
    if !ci::controls() {
        return ::core::ops::ControlFlow::Continue(());
    }
    ::core::ops::ControlFlow::Break(false)
}
