// DirectX 12 fix for Horizon's terrain shader (not part of Cinematic Interactions itself).
// terrain.wgsl's `packed_corner` writes `pos[axis]`, a vector element picked at run time. On DX12
// the shader goes through FXC, which cannot write a vector element at a run-time index ("error
// X3500: array reference cannot be used as an l-value"), so "Terrain solid (flat)" fails and the
// game crashes while loading. This builds the same position with per-axis masks instead, and only
// for the "Horizon terrain" module; Vulkan and OpenGL get exactly the same result as before.
fn cinematic_dx12_terrain(this: &On<'_>, descriptor: &wgpu::ShaderModuleDescriptor<'_>) -> ::core::ops::ControlFlow<wgpu::ShaderModule> {
    const OLD: &str = "    var pos = block;
    pos[axis] = block[axis] + side;
    pos[(axis + 1u) % 3u] = block[(axis + 1u) % 3u] + du;
    pos[(axis + 2u) % 3u] = block[(axis + 2u) % 3u] + dw;
";
    const NEW: &str = "    let axis1 = (axis + 1u) % 3u;
    let axis2 = (axis + 2u) % 3u;
    let pos = block
        + side * vec3(u32(axis == 0u), u32(axis == 1u), u32(axis == 2u))
        + du * vec3(u32(axis1 == 0u), u32(axis1 == 1u), u32(axis1 == 2u))
        + dw * vec3(u32(axis2 == 0u), u32(axis2 == 1u), u32(axis2 == 2u));
";
    if descriptor.label != Some("Horizon terrain") {
        return ::core::ops::ControlFlow::Continue(());
    }
    let wgpu::ShaderSource::Wgsl(source) = &descriptor.source else {
        return ::core::ops::ControlFlow::Continue(());
    };
    // Already patched (this hook runs again for the patched module), or Horizon changed the code.
    if !source.contains(OLD) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let patched = source.replace(OLD, NEW);
    ::core::ops::ControlFlow::Break(this.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: descriptor.label,
        source: wgpu::ShaderSource::Wgsl(patched.into()),
    }))
}
