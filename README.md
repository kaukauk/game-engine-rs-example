# game-engine-rs-example

A small 3D rendering example in Rust built on wgpu and winit. It opens a window, draws a ground plane and a coloured cube, and lets you move around with a first-person camera.

## What it does

- Sets up a wgpu device, a surface and a render pipeline with a WGSL shader (`shaders/shader.wgsl`) that uses per-vertex colours.
- Renders hard-coded meshes: a ground plane and a cube with coloured vertices (`src/graphics/mesh.rs`).
- Perspective camera built with cgmath (`src/graphics/camera.rs`).
- First-person controls: WASD to move, mouse to look. The cursor is hidden and confined to the window.
- Handles window resizing and surface loss.

## Project layout

```
src/main.rs               window creation and event loop (winit)
src/engine.rs             input handling, camera movement, update/render calls
src/graphics/renderer.rs  wgpu setup, buffers, pipeline, draw calls
src/graphics/camera.rs    camera and view-projection matrix
src/graphics/mesh.rs      vertex layout and cube/plane mesh data
shaders/shader.wgsl       vertex and fragment shaders
```

## Tech

- Rust (2021 edition)
- wgpu 0.15, winit 0.28, cgmath, bytemuck, pollster, env_logger

## Running it

Requires a Rust toolchain and a GPU or driver that wgpu supports (Vulkan, Metal, DX12 or OpenGL).

```
cargo run --release
```

Close the window to exit. No key is bound to quit.

## Notes

This is an early experiment, not a full engine. There is no depth buffer, and the plane and cube share a single uniform buffer for their model matrices, so both draws end up using the last transform written. `tokio` and `log` are listed as dependencies but are not used yet.
