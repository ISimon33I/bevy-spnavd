use std::sync::Mutex;
use std::thread;

use bevy::prelude::*;
use spnav_rs::{SpnavClient, SpnavEvent};
use tokio::sync::mpsc;

// ============================================================================
// Plugin
// ============================================================================

/// Bevy plugin for SpaceMouse via spnavd.
pub struct SpnavdPlugin;

impl Plugin for SpnavdPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        app.insert_resource(SpaceMouseState::default())
            .insert_resource(SpaceMouseSettings::default())
            .insert_resource(SpaceMouseReceiver {
                receiver: Mutex::new(receiver),
            })
            .add_systems(
                Update,
                (
                    receive_spacemouse_events,
                    update_spacemouse_state,
                    spacemouse_camera_system,
                )
                    .chain(),
            );

        spawn_spacemouse_thread(sender);
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Settings for SpaceMouse.
#[derive(Resource, Debug, Clone)]
pub struct SpaceMouseSettings {
    /// Multiplier for translation.
    pub translation_sensitivity: f32,

    /// Multiplier for rotation.
    pub rotation_sensitivity: f32,
    
    /// Translation acceleration curve
    pub translation_curve: f32,
    
    /// Rotation acceleration curve
    pub rotation_curve: f32,
    
    /// Smoothing lerp speed
    pub smooth_speed: f32,
    
    /// Whether to invert the Y-axis for translation.
    pub invert_translation_y: bool,

    /// Whether to invert rotation X.
    pub invert_rotation_x: bool,

    /// Whether to invert rotation Y.
    pub invert_rotation_y: bool,

    /// Whether to invert rotation Z.
    pub invert_rotation_z: bool,

    /// Whether to invert translation X.
    pub invert_translation_x: bool,

    /// Whether to invert translation Z.
    pub invert_translation_z: bool,
}

impl Default for SpaceMouseSettings {
    fn default() -> Self {
        Self {
            translation_sensitivity: 10.0,
            rotation_sensitivity: 5.0,
            
            translation_curve: 0.,
            rotation_curve: 0.,
            
            smooth_speed: 12.0,

            invert_translation_x: false,
            invert_translation_y: false,
            invert_translation_z: false,

            invert_rotation_x: false,
            invert_rotation_y: false,
            invert_rotation_z: false,
        }
    }
}

// ============================================================================
// State
// ============================================================================

/// Current state from SpaceMouse.
///
/// `translation` and `rotation` contain the latest input.
///
/// `buttons` contains the current button state.
///
/// `just_pressed` and `just_released` are bitmasks for buttons that
/// changed during the last Bevy frame.
#[derive(Resource, Debug)]
pub struct SpaceMouseState {
    /// Translation from SpaceMouse.
    ///
    /// X = left/right
    /// Y = up/down
    /// Z = forward/backward
    pub translation: Vec3,
    pub translation_raw: Vec3,

    /// Rotation from SpaceMouse.
    ///
    /// X = pitch
    /// Y = yaw
    /// Z = roll
    pub rotation: Vec3,
    pub rotation_raw: Vec3,

    /// Current buttons.
    ///
    /// Bit 0 = button 0
    /// Bit 1 = button 1
    /// etc.
    pub buttons: u32,

    /// Buttons that were pressed this frame.
    pub just_pressed: u32,

    /// Buttons that were released this frame.
    pub just_released: u32,

    /// Whether the SpaceMouse device is connected.
    pub connected: bool,
}

impl Default for SpaceMouseState {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            translation_raw: Vec3::ZERO,
            rotation: Vec3::ZERO,
            rotation_raw: Vec3::ZERO,

            buttons: 0,
            just_pressed: 0,
            just_released: 0,

            connected: false,
        }
    }
}

impl SpaceMouseState {
    /// Is the button pressed?
    pub fn pressed(&self, button: u8) -> bool {
        if button >= 32 {
            return false;
        }

        self.buttons & (1 << button) != 0
    }

    /// Was the button pressed this frame?
    pub fn just_pressed(&self, button: u8) -> bool {
        if button >= 32 {
            return false;
        }

        self.just_pressed & (1 << button) != 0
    }

    /// Was the button released this frame?
    pub fn just_released(&self, button: u8) -> bool {
        if button >= 32 {
            return false;
        }

        self.just_released & (1 << button) != 0
    }
}

// ============================================================================
// Receiver
// ============================================================================

#[derive(Resource)]
struct SpaceMouseReceiver {
    receiver: Mutex<mpsc::UnboundedReceiver<SpnavEvent>>,
}

// ============================================================================
// Thread
// ============================================================================

fn spawn_spacemouse_thread(
    sender: mpsc::UnboundedSender<SpnavEvent>,
) {
    thread::Builder::new()
        .name("spacemouse".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime,

                Err(error) => {
                    eprintln!(
                        "SpaceMouse: could not create Tokio runtime: {error}"
                    );
                    return;
                }
            };

            runtime.block_on(async move {
                let mut client = match SpnavClient::open().await {
                    Ok(client) => client,

                    Err(error) => {
                        eprintln!(
                            "SpaceMouse: could not connect to spacenavd: {error}"
                        );

                        return;
                    }
                };

                loop {
                    match client.wait_event().await {
                        Ok(event) => {
                            if sender.send(event).is_err() {
                                // Bevy has shut down.
                                break;
                            }
                        }

                        Err(error) => {
                            eprintln!(
                                "SpaceMouse: spnav error: {error}"
                            );

                            break;
                        }
                    }
                }
            });
        })
        .expect("could not create SpaceMouse thread");
}

// ============================================================================
// Receive events
// ============================================================================

fn receive_spacemouse_events(
    receiver: Res<SpaceMouseReceiver>,
    mut state: ResMut<SpaceMouseState>,
) {
    let mut receiver = receiver.receiver.lock().unwrap();

    // These are frame-based and should be reset every frame.
    state.just_pressed = 0;
    state.just_released = 0;

    while let Ok(event) = receiver.try_recv() {
        match event {
            SpnavEvent::Motion(event) => {
                state.translation_raw = Vec3::new(
                    event.x as f32,
                    event.y as f32,
                    event.z as f32,
                );

                state.rotation_raw = Vec3::new(
                    event.rx as f32,
                    event.ry as f32,
                    event.rz as f32,
                );

                state.connected = true;
            }

            SpnavEvent::Button(event) => {
                let button = event.bnum;

                if !(0..32).contains(&button) {
                    continue;
                }

                let button = button as u8;
                let mask = 1u32 << button;

                if event.press {
                    if state.buttons & mask == 0 {
                        state.just_pressed |= mask;
                    }

                    state.buttons |= mask;
                } else {
                    if state.buttons & mask != 0 {
                        state.just_released |= mask;
                    }

                    state.buttons &= !mask;
                }

                state.connected = true;
            }

            // Other event types don't need to be exposed to the game.
            _ => {}
        }
    }
}

// ============================================================================
// Process input
// ============================================================================
fn update_spacemouse_state(
    mut state: ResMut<SpaceMouseState>,
    settings: Res<SpaceMouseSettings>,
    time: Res<Time>,
) {
    let axis_scaling_factor: f32 = 1.0 / 150.0;
    let mut translation = state.translation_raw * axis_scaling_factor * settings.translation_sensitivity;
    let mut rotation = state.rotation_raw * axis_scaling_factor * settings.rotation_sensitivity;
    translation *= translation.abs().powf(settings.translation_curve);
    rotation *= rotation.abs().powf(settings.rotation_curve);

    if settings.invert_translation_x {
        translation.x *= -1.0;
    }

    if settings.invert_translation_y {
        translation.y *= -1.0;
    }

    if settings.invert_translation_z {
        translation.z *= -1.0;
    }

    if settings.invert_rotation_x {
        rotation.x *= -1.0;
    }

    if settings.invert_rotation_y {
        rotation.y *= -1.0;
    }

    if settings.invert_rotation_z {
        rotation.z *= -1.0;
    }
    
    state.translation = state.translation.lerp(translation, time.delta_secs()*settings.smooth_speed);
    state.rotation = state.rotation.lerp(rotation, time.delta_secs()*settings.smooth_speed);
}

// ============================================================================
// Camera controller
// ============================================================================

/// Component that enables SpaceMouse control on a camera.
#[derive(Component, Debug, Clone)]
pub struct SpaceMouseCameraController {
    /// Translation speed multiplier.
    pub translation_speed: f32,

    /// Rotation speed multiplier.
    pub rotation_speed: f32,

    /// Whether the camera should move relative to its local rotation.
    pub local_translation: bool,

    /// Whether the camera should rotate locally.
    pub local_rotation: bool,
}

impl Default for SpaceMouseCameraController {
    fn default() -> Self {
        Self {
            translation_speed: 1.0,
            rotation_speed: 1.0,

            local_translation: true,
            local_rotation: true,
        }
    }
}

/// System that applies SpaceMouse input to cameras.
fn spacemouse_camera_system(
    time: Res<Time>,
    state: Res<SpaceMouseState>,
    mut cameras: Query<(
        &SpaceMouseCameraController,
        &mut Transform,
    )>,
) {
    let dt = time.delta_secs();

    for (controller, mut transform) in &mut cameras {
        // ------------------------------------------------------------
        // Translation
        // ------------------------------------------------------------

        let mut translation =
            state.translation * controller.translation_speed * dt;

        if controller.local_translation {
            // SpaceMouse data is in its own coordinate convention.
            //
            // Here it is used as:
            // X -> local X
            // Y -> local Y
            // Z -> local Z
            //
            // Negate Z because Bevy cameras look along -Z.
            translation.z *= -1.0;

            let translation_delta = transform.rotation * translation;
            transform.translation += translation_delta
        } else {
            translation.z *= -1.0;

            transform.translation += translation;
        }

        // ------------------------------------------------------------
        // Rotation
        // ------------------------------------------------------------

        let rotation =
            state.rotation * controller.rotation_speed * dt;

        if rotation == Vec3::ZERO {
            continue;
        }

        let pitch = Quat::from_rotation_x(rotation.x);
        let yaw = Quat::from_rotation_y(rotation.y);
        let roll = Quat::from_rotation_z(rotation.z);

        let delta = if controller.local_rotation {
            // Local rotation.
            pitch * roll * yaw
            //yaw * pitch * roll
        } else {
            // World-space rotation.
            yaw * pitch * roll
        };

        transform.rotation = transform.rotation * delta;
    }
}

// ============================================================================
// Plugin extension
// ============================================================================

impl SpnavdPlugin {
    /// Creates the plugin with default settings.
    pub fn new() -> Self {
        Self
    }
}
