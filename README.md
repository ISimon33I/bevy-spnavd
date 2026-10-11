# bevy-spnavd

A Bevy plugin for using SpaceMouse devices via `spacenavd`.

## Features

- Automatic SpaceMouse input handling in a background thread
- Smooth, interpolated output with configurable sensitivity and curves
- Support for button events
- Camera controller component for easy integration

## Requirements

- `spnavd` must be running on your system. Install it via your package manager:
  - **Ubuntu/Debian**: `sudo apt install spacenavd`

## Usage

Run:

```shell
cargo add bevy-spnavd
```

Example setup in your Bevy app:

```rust
use bevy::prelude::*;
use bevy_spnavd::{SpnavdPlugin, SpaceMouseCameraController};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpnavdPlugin)
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 5.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert_resource(SpaceMouseCameraController {
            translation_speed: 10.0,
            rotation_speed: 2.0,
            local_translation: true,
            local_rotation: true,
        })
        .run();
}
```

## Configuration

### SpaceMouseSettings

Access the `SpaceMouseSettings` resource to configure sensitivity and axis inversion:

```rust
// In a system:
fn control_system(settings: Res<SpaceMouseSettings>) {
    info!("Translation sensitivity: {}", settings.translation_sensitivity);
}
```

Available settings:
- `translation_sensitivity`: Multiplier for translation (default: 10.0)
- `rotation_sensitivity`: Multiplier for rotation (default: 5.0)
- `translation_curve`: Translation acceleration curve
- `rotation_curve`: Rotation acceleration curve  
- `smooth_speed`: Smoothing LERP speed
- Inversion flags for all axes (X, Y, Z for both translation and rotation)

### SpaceMouseState

Access current input state:

```rust
fn print_spacemouse(state: Res<SpaceMouseState>) {
    info!("Translation: {:?}", state.translation);
    info!("Rotation: {:?}", state.rotation);
    info!("Buttons pressed: {}", state.buttons);
}
```

### Button Events

Check button presses in your systems:

```rust
fn handle_buttons(mut state: ResMut<SpaceMouseState>) {
    if state.just_pressed(0) {
        info!("Button 0 was just pressed!");
    }
    
    if state.pressed(1) {
        info!("Button 1 is currently held down");
    }
}
```

### Camera Controller

Add `SpaceMouseCameraController` component to any camera entity:

```rust
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        SpaceMouseCameraController::default(),
    ));
}
```