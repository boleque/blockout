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
struct Figure {
    position: Vec3i,
    blocks: Vec<Vec3i>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Well {
    width: i32,
    height: i32,
    depth: i32,
    occupied: Vec<Vec3i>,
}

impl Well {
    fn contains(&self, position: Vec3i) -> bool {
        position.x >= 0
            && position.x < self.width
            && position.y >= 0
            && position.y < self.height
            && position.z >= 0
            && position.z < self.depth
    }

    fn contains_figure(&self, active_figure: &Figure) -> bool {
        for local_block in &active_figure.blocks {
            let world_block = active_figure.world_position(*local_block);

            if !self.contains(world_block) || self.is_occupied(world_block) {
                return false;
            }
        }

        true
    }

    fn is_occupied(&self, position: Vec3i) -> bool {
        self.occupied.contains(&position)
    }

    fn lock_figure(&mut self, active_figure: &Figure) {
        for local_block in &active_figure.blocks {
            let world_block = active_figure.world_position(*local_block);
            self.occupied.push(world_block);
        }
    }
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

impl Figure {
    fn new() -> Self {
        Self {
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
        }
    }

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
    active_figure: Figure,
    show_line: bool,
    game_over: bool,
}

#[derive(Component)]
struct DebugLine;

#[derive(Component)]
struct FigureBlockIndex {
    index: usize,
}

#[derive(Component)]
struct LockedBlock;

#[derive(Component)]
struct GameOverText;

fn main() {
    let game = GameModel {
        well: Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        },
        active_figure: Figure::new(),
        show_line: true,
        game_over: false,
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
        .add_systems(
            Update,
            (handle_input, sync_figure_position, sync_game_over_text).chain(),
        )
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

    commands.spawn((
        Text::new("GAME OVER"),
        TextFont {
            font_size: FontSize::Px(48.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.2, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: px(250.0),
            left: px(450.0),
            ..default()
        },
        Visibility::Hidden,
        GameOverText,
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

    for (index, local_block) in game.active_figure.blocks.iter().enumerate() {
        let world_block = game.active_figure.world_position(*local_block);

        let entity = (
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(block_material.clone()),
            Transform::from_xyz(
                world_block.x as f32,
                world_block.y as f32,
                world_block.z as f32,
            ),
            FigureBlockIndex { index },
        );

        commands.spawn(entity);

        info!("local {local_block:?} -> world {world_block:?}");
    }
}

fn handle_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameModel>,
    mut line: Query<&mut Visibility, With<DebugLine>>,
    figure_blocks: Query<(
        &FigureBlockIndex,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if game.game_over {
        return;
    }

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
        let mut candidate = game.active_figure.clone();
        candidate.move_by(delta);

        if game.well.contains_figure(&candidate) {
            game.active_figure = candidate;
            info!("active_figure position: {:?}", game.active_figure.position);
        } else {
            info!("movement blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyX) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::X);

        if game.well.contains_figure(&candidate) {
            game.active_figure = candidate;
            info!("rotate X: {:?}", game.active_figure.blocks);
        } else {
            info!("rotation X blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyY) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::Y);

        if game.well.contains_figure(&candidate) {
            game.active_figure = candidate;
            info!("rotate Y: {:?}", game.active_figure.blocks);
        } else {
            info!("rotation Y blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyZ) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::Z);

        if game.well.contains_figure(&candidate) {
            game.active_figure = candidate;
            info!("rotate Z: {:?}", game.active_figure.blocks);
        } else {
            info!("rotation Z blocked by well bounds");
        }
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

    if keyboard.just_pressed(KeyCode::Enter) {
        let drop_delta = Vec3i { x: 0, y: 0, z: 1 };

        loop {
            let mut candidate = game.active_figure.clone();
            candidate.move_by(drop_delta);

            if game.well.contains_figure(&candidate) {
                game.active_figure = candidate;
            } else {
                break;
            }
        }

        let locked_figure = game.active_figure.clone();
        game.well.lock_figure(&locked_figure);

        info!("active_figure locked at {:?}", game.active_figure.position);
        info!("occupied cells: {:?}", game.well.occupied);

        for (block_index, mesh, material) in &figure_blocks {
            let local_block = locked_figure.blocks[block_index.index];
            let world_block = locked_figure.world_position(local_block);

            commands.spawn((
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(material.0.clone()),
                Transform::from_xyz(
                    world_block.x as f32,
                    world_block.y as f32,
                    world_block.z as f32,
                ),
                LockedBlock,
            ));
        }
        game.active_figure = Figure::new();

        let can_spawn = game.well.contains_figure(&game.active_figure);
        if !can_spawn {
            game.game_over = true;
        }
    }
}

fn sync_figure_position(
    game: Res<GameModel>,
    mut blocks: Query<(&FigureBlockIndex, &mut Transform)>,
) {
    for (block, mut transform) in &mut blocks {
        let local_block = game.active_figure.blocks[block.index];
        let world_block = game.active_figure.world_position(local_block);

        transform.translation = Vec3::new(
            world_block.x as f32,
            world_block.y as f32,
            world_block.z as f32,
        );
    }
}

fn sync_game_over_text(game: Res<GameModel>, mut text: Query<&mut Visibility, With<GameOverText>>) {
    for mut visibility in &mut text {
        *visibility = if game.game_over {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locking_figure_marks_its_world_cells_as_occupied() {
        let mut well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        };

        let active_figure = Figure {
            position: Vec3i { x: 2, y: 3, z: 5 },
            blocks: vec![Vec3i { x: 0, y: 0, z: 0 }, Vec3i { x: 1, y: 0, z: 0 }],
        };

        well.lock_figure(&active_figure);

        assert!(well.is_occupied(Vec3i { x: 2, y: 3, z: 5 }));
        assert!(well.is_occupied(Vec3i { x: 3, y: 3, z: 5 }));
        assert_eq!(well.occupied.len(), 2);
    }

    #[test]
    fn well_rejects_figure_overlapping_occupied_cell() {
        let well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: vec![Vec3i { x: 3, y: 3, z: 0 }],
        };

        let active_figure = Figure {
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![Vec3i { x: 0, y: 0, z: 0 }, Vec3i { x: 1, y: 0, z: 0 }],
        };

        assert!(!well.contains_figure(&active_figure));
    }

    #[test]
    fn well_contains_figure_using_world_positions() {
        let well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        };

        let mut active_figure = Figure {
            position: Vec3i { x: 3, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
        };

        assert!(well.contains_figure(&active_figure));

        active_figure.position.x += 1;

        assert!(!well.contains_figure(&active_figure));
    }

    #[test]
    fn well_contains_only_positions_inside_bounds() {
        let well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        };

        assert!(well.contains(Vec3i { x: 0, y: 0, z: 0 }));
        assert!(well.contains(Vec3i { x: 4, y: 4, z: 11 }));

        assert!(!well.contains(Vec3i { x: -1, y: 0, z: 0 }));
        assert!(!well.contains(Vec3i { x: 5, y: 0, z: 0 }));

        assert!(!well.contains(Vec3i { x: 0, y: -1, z: 0 }));
        assert!(!well.contains(Vec3i { x: 0, y: 5, z: 0 }));

        assert!(!well.contains(Vec3i { x: 0, y: 0, z: -1 }));
        assert!(!well.contains(Vec3i { x: 0, y: 0, z: 12 }));
    }

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
    fn four_rotations_restore_figure() {
        let original = Figure {
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
                "four rotations around {axis:?} must restore the active_figure"
            );
        }
    }
}
