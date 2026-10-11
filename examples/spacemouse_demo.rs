use bevy::prelude::*;
use bevy_spnavd::{SpnavdPlugin, SpaceMouseCameraController};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpnavdPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the camera with SpaceMouse controller
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 5.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
        SpaceMouseCameraController::default(),
    ));

    // Spawn a cube to look at
    commands.spawn((
        Transform::from_translation(Vec3::ZERO),
        Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
             base_color: Color::linear_rgb(0.8, 0.7, 0.6),
             ..default()
        })),
    ));

    // Spawn a light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}