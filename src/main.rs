use bevy::{prelude::*, text::FontSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vec3i {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Piece {
    position: Vec3i,
    blocks: Vec<Vec3i>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Well {
    width: i32,
    height: i32,
    depth: i32,
}

impl Vec3i {
    fn rotated_90(self, axis: Axis) -> Self {
        match axis {
            Axis::X => Self {
                x: self.x,
                y: -self.z,
                z: self.y,
            },
            Axis::Y => Self {
                x: self.z,
                y: self.y,
                z: -self.x,
            },
            Axis::Z => Self {
                x: -self.y,
                y: self.x,
                z: self.z,
            },
        }
    }
}

impl Piece {
    fn world_position(&self, local_block: Vec3i) -> Vec3i {
        Vec3i {
            x: self.position.x + local_block.x,
            y: self.position.y + local_block.y,
            z: self.position.z + local_block.z,
        }
    }

    fn move_by(&mut self, delta: Vec3i) {
        self.position.x += delta.x;
        self.position.y += delta.y;
        self.position.z += delta.z;
    }

    fn rotate_90(&mut self, axis: Axis) {
        for block in &mut self.blocks {
            *block = (*block).rotated_90(axis);
        }
    }
}

#[derive(Resource)]
struct GameModel {
    well: Well,
    piece: Piece,
    show_line: bool,
}

#[derive(Component)]
struct DebugLine;

#[derive(Component)]
struct PieceBlock {
    index: usize,
}

fn main() {
    let game = GameModel {
        well: Well {
            width: 5,
            height: 5,
            depth: 12,
        },
        piece: Piece {
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
        },
        show_line: true,
    };

    App::new()
        .insert_resource(game)
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Blockout".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, sync_piece_visuals).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    game: Res<GameModel>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 8.0, -14.0).looking_at(Vec3::new(2.5, 2.5, 4.0), Vec3::Y),
        IsDefaultUiCamera,
    ));

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, -4.0),
    ));

    commands.spawn((
        Text::new("MOVE: A/D = X, W/S = Y, E = +Z\nROTATE: X / Y / Z, SPACE = line"),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(20.0),
            left: px(20.0),
            ..default()
        },
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(100.0),
            left: px(100.0),
            width: px(200.0),
            height: px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.0, 1.0, 0.0)),
        DebugLine,
    ));

    info!(
        "well size: {} x {} x {}",
        game.well.width, game.well.height, game.well.depth
    );

    let block_mesh = meshes.add(Cuboid::new(0.9, 0.9, 0.9));

    let block_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.2, 0.7, 1.0),
        metallic: 0.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    for (index, local_block) in game.piece.blocks.iter().enumerate() {
        let world_block = game.piece.world_position(*local_block);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(block_material.clone()),
            Transform::from_xyz(
                world_block.x as f32,
                world_block.y as f32,
                world_block.z as f32,
            ),
            PieceBlock { index },
        ));

        info!("local {local_block:?} -> world {world_block:?}");
    }
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameModel>,
    mut line: Query<&mut Visibility, With<DebugLine>>,
) {
    let mut delta = Vec3i { x: 0, y: 0, z: 0 };

    if keyboard.just_pressed(KeyCode::KeyA) {
        delta.x -= 1;
    }
    if keyboard.just_pressed(KeyCode::KeyD) {
        delta.x += 1;
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        delta.y -= 1;
    }
    if keyboard.just_pressed(KeyCode::KeyW) {
        delta.y += 1;
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        delta.z += 1;
    }

    if delta.x != 0 || delta.y != 0 || delta.z != 0 {
        game.piece.move_by(delta);
        info!("piece position: {:?}", game.piece.position);
    }

    if keyboard.just_pressed(KeyCode::KeyX) {
        game.piece.rotate_90(Axis::X);
        info!("rotate X: {:?}", game.piece.blocks);
    }
    if keyboard.just_pressed(KeyCode::KeyY) {
        game.piece.rotate_90(Axis::Y);
        info!("rotate Y: {:?}", game.piece.blocks);
    }
    if keyboard.just_pressed(KeyCode::KeyZ) {
        game.piece.rotate_90(Axis::Z);
        info!("rotate Z: {:?}", game.piece.blocks);
    }

    if keyboard.just_pressed(KeyCode::Space) {
        game.show_line = !game.show_line;

        for mut visibility in &mut line {
            *visibility = if game.show_line {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn sync_piece_visuals(game: Res<GameModel>, mut blocks: Query<(&PieceBlock, &mut Transform)>) {
    for (block, mut transform) in &mut blocks {
        let local_block = game.piece.blocks[block.index];
        let world_block = game.piece.world_position(local_block);

        transform.translation = Vec3::new(
            world_block.x as f32,
            world_block.y as f32,
            world_block.z as f32,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_order_matters() {
        let original = Vec3i { x: 1, y: 2, z: 3 };

        let x_then_y = original.rotated_90(Axis::X).rotated_90(Axis::Y);

        let y_then_x = original.rotated_90(Axis::Y).rotated_90(Axis::X);

        assert_eq!(x_then_y, Vec3i { x: 2, y: -3, z: -1 });
        assert_eq!(y_then_x, Vec3i { x: 3, y: 1, z: 2 });
        assert_ne!(x_then_y, y_then_x);
    }

    #[test]
    fn four_rotations_restore_piece() {
        let original = Piece {
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
        };

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let mut rotated = original.clone();

            for _ in 0..4 {
                rotated.rotate_90(axis);
            }

            assert_eq!(
                rotated, original,
                "four rotations around {axis:?} must restore the piece"
            );
        }
    }
}
