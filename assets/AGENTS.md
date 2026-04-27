# Asset Generation Notes

This folder contains runtime assets served by Trunk and loaded by Bevy.

## Sprite Workflow

Use the `imagegen` skill for generated bitmap assets. For item/UI sprites, generate one asset at a time instead of requesting a sprite sheet.

Recommended prompt shape:

```text
Use case: stylized-concept
Asset type: 32x32 game inventory sprite source
Primary request: one isolated <asset name> sprite, <short visual description>.
Input images: use the attached reference image only as style inspiration for compact readable pixel-game item sprites.
Scene/backdrop: perfectly flat solid #00ff00 chroma-key background for removal; one uniform color with no shadows, gradients, texture, floor plane, or lighting variation.
Subject: <subject details and key material/color cues>.
Style/medium: pixel-art-inspired game item icon with hard-edged facets, dark outline, simple upper-left highlights, built to remain clear after hard downscaling to 32x32.
Composition/framing: single centered object, three-quarter view, fills about 65-75% of the canvas, generous padding, not cropped.
Constraints: one asset only, no spritesheet, no text, no watermark, no UI frame, no cast shadow, no reflection, do not use #00ff00 anywhere in the subject.
```

Use a different key color only if the subject itself needs green. Keep the background flat so it can be removed locally.

## Post Processing

Generated images are saved by Codex under:

```text
/mnt/c/Users/golde/.codex/generated_images/<generation-id>/<image-id>.png
```

For project assets:

1. Copy or read the generated source image from the Codex generated image folder.
2. Remove the chroma-key background and write an alpha PNG.
3. Crop to the non-transparent content with a small margin.
4. Resize to exactly `32x32` using nearest-neighbor sampling.
5. Save item sprites under `assets/sprites/items/`.
6. Save UI sprites under `assets/sprites/ui/`.
7. Verify with `file assets/sprites/items/*.png assets/sprites/ui/*.png`; each final sprite should be `32 x 32`, `RGBA`, PNG.

Do not smooth-scale final sprites. The final downscale must be nearest-neighbor/hard scaling so the icons stay crisp in Bevy.

## Bevy And Trunk

Bevy asset paths are relative to the `assets/` folder. For example:

```rust
asset_server.load("sprites/items/stone.png")
```

Do not include `assets/` in the Bevy load path.

Trunk must copy this directory:

```html
<link data-trunk rel="copy-dir" href="assets" />
```

The app disables Bevy asset metadata checks with:

```rust
AssetPlugin {
    meta_check: AssetMetaCheck::Never,
    ..default()
}
```

Keep that setting unless real `.meta` files are added. Without it, Trunk can return `index.html` for missing `.meta` URLs, and Bevy may log metadata parse errors in the browser.

Use `ImagePlugin::default_nearest()` so scaled sprites render with nearest sampling.

## Adding A New Sprite

After saving the new `32x32` PNG:

1. Add a field to `UiIconAssets` if the sprite needs a persistent UI handle.
2. Load it with `asset_server.load("sprites/.../<name>.png")`.
3. Add or update any item-to-icon mapping, such as `UiIconAssets::for_item`.
4. Run `cargo check --target wasm32-unknown-unknown`.
5. If possible, open `http://127.0.0.1:8080/` and check the browser console for asset load errors.
