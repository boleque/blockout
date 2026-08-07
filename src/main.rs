use bevy::{prelude::*, text::FontSize};
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FigureColor {
    Cyan,
    Orange,
    Green,
    Purple,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockMaterial {
    Metal,
    Rubber,
    Crystal,
    Neon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FigureKind {
    I,
    O,
    T,
    L,
    J,
    S,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vec3i {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Resource)]
struct BlockVisualAssets {
    mesh: Handle<Mesh>,
    cyan: Handle<StandardMaterial>,
    orange: Handle<StandardMaterial>,
    green: Handle<StandardMaterial>,
    purple: Handle<StandardMaterial>,
    yellow: Handle<StandardMaterial>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Figure {
    kind: FigureKind,
    position: Vec3i,
    blocks: Vec<Vec3i>,
    color: FigureColor,
}

#[derive(Debug)]
struct FigureBag {
    figures: Vec<Figure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Well {
    width: i32,
    height: i32,
    depth: i32,
    occupied: Vec<Vec3i>,
}

#[derive(Resource)]
struct GameModel {
    well: Well,
    active_figure: Figure,
    next_figure: Figure,
    show_line: bool,
    game_over: bool,
    figure_bag: FigureBag,
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

#[derive(Component)]
struct PreviewBlockIndex {
    index: usize,
}

#[derive(Resource)]
struct GravityTimer {
    timer: Timer,
}

impl FigureBag {
    fn new() -> Self {
        let mut bag = Self {
            figures: Vec::new(),
        };
        bag.refill();

        bag
    }

    fn refill(&mut self) {
        let kinds = vec![
            FigureKind::I,
            FigureKind::O,
            FigureKind::T,
            FigureKind::L,
            FigureKind::J,
            FigureKind::S,
            FigureKind::Z,
        ];

        let colors = vec![
            FigureColor::Cyan,
            FigureColor::Orange,
            FigureColor::Green,
            FigureColor::Purple,
            FigureColor::Yellow,
        ];

        self.figures = kinds
            .into_iter()
            .zip(colors.into_iter().cycle())
            .map(|(kind, color)| Figure::new(kind, color))
            .collect();

        let mut rng = rand::rng();
        self.figures.shuffle(&mut rng);
    }

    fn next_figure(&mut self) -> Figure {
        if self.figures.is_empty() {
            self.refill();
        }

        self.figures
            .pop()
            .expect("figure bag must contain a figure after refill")
    }
}

impl BlockVisualAssets {
    fn material_for(&self, color: FigureColor) -> Handle<StandardMaterial> {
        match color {
            FigureColor::Cyan => self.cyan.clone(),
            FigureColor::Orange => self.orange.clone(),
            FigureColor::Green => self.green.clone(),
            FigureColor::Purple => self.purple.clone(),
            FigureColor::Yellow => self.yellow.clone(),
        }
    }
}

impl FigureColor {
    fn next(self) -> FigureColor {
        match self {
            FigureColor::Cyan => FigureColor::Orange,
            FigureColor::Orange => FigureColor::Green,
            FigureColor::Green => FigureColor::Purple,
            FigureColor::Purple => FigureColor::Yellow,
            FigureColor::Yellow => FigureColor::Cyan,
        }
    }
}

impl FigureKind {
    fn next(self) -> FigureKind {
        match self {
            FigureKind::I => FigureKind::O,
            FigureKind::O => FigureKind::T,
            FigureKind::T => FigureKind::L,
            FigureKind::L => FigureKind::J,
            FigureKind::J => FigureKind::S,
            FigureKind::S => FigureKind::Z,
            FigureKind::Z => FigureKind::I,
        }
    }
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

    fn can_place_figure(&self, active_figure: &Figure) -> bool {
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

    fn is_plane_full(&self, z: i32) -> bool {
        if z < 0 || z >= self.depth {
            return false;
        }

        for x in 0..self.width {
            for y in 0..self.height {
                if !self.is_occupied(Vec3i { x, y, z }) {
                    return false;
                }
            }
        }

        true
    }

    fn clear_plane(&mut self, z: i32) -> bool {
        if !self.is_plane_full(z) {
            return false;
        }

        self.occupied.retain(|position| position.z != z);

        for position in &mut self.occupied {
            if position.z < z {
                position.z += 1;
            }
        }

        true
    }

    fn clear_full_planes(&mut self) -> usize {
        let mut cleared_planes = 0;
        let mut z = self.depth - 1;

        while z >= 0 {
            if self.clear_plane(z) {
                cleared_planes += 1;
            } else {
                z -= 1;
            }
        }

        cleared_planes
    }
}

impl GameModel {
    fn new() -> Self {
        let mut figure_bag = FigureBag::new();
        let active_figure = figure_bag.next_figure();
        let next_figure = figure_bag.next_figure();

        Self {
            well: Well {
                width: 5,
                height: 5,
                depth: 12,
                occupied: Vec::new(),
            },
            active_figure: active_figure,
            next_figure: next_figure,
            show_line: true,
            game_over: false,
            figure_bag: figure_bag,
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
    fn new(kind: FigureKind, color: FigureColor) -> Self {
        let blocks = match kind {
            FigureKind::I => vec![
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 2, y: 0, z: 0 },
            ],
            FigureKind::O => vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
            FigureKind::T => vec![
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
            ],
            FigureKind::L => vec![
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
            FigureKind::J => vec![
                Vec3i { x: -1, y: 1, z: 0 },
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
            ],
            FigureKind::S => vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: -1, y: 1, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
            ],
            FigureKind::Z => vec![
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
        };
        Self {
            kind: kind,
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: blocks,
            color,
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

fn make_block_material(base_color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color,
        metallic: 0.0,
        perceptual_roughness: 0.25,
        ..default()
    }
}

fn main() {
    App::new()
        .insert_resource(GameModel::new())
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .insert_resource(GravityTimer {
            timer: Timer::from_seconds(0.7, TimerMode::Repeating),
        })
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
            (
                handle_input,
                apply_gravity,
                sync_figure_position,
                sync_next_figure_preview,
                sync_game_over_text,
            )
                .chain(),
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

    let block_visuals = BlockVisualAssets {
        mesh: meshes.add(Cuboid::new(0.9, 0.9, 0.9)),
        cyan: materials.add(make_block_material(Color::srgb(0.2, 0.8, 1.0))),
        orange: materials.add(make_block_material(Color::srgb(1.0, 0.4, 0.1))),
        green: materials.add(make_block_material(Color::srgb(0.2, 0.9, 0.3))),
        purple: materials.add(make_block_material(Color::srgb(0.7, 0.2, 1.0))),
        yellow: materials.add(make_block_material(Color::srgb(1.0, 0.85, 0.1))),
    };

    let block_mesh = block_visuals.mesh.clone();

    let block_material = block_visuals.material_for(game.active_figure.color);
    let preview_material = block_visuals.material_for(game.next_figure.color);

    commands.insert_resource(block_visuals);

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

    for (index, local_block) in game.next_figure.blocks.iter().enumerate() {
        let preview_scale = 0.7;

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(preview_material.clone()),
            Transform::from_xyz(
                7.0 + local_block.x as f32 * preview_scale,
                3.0 + local_block.y as f32 * preview_scale,
                local_block.z as f32 * preview_scale,
            )
            .with_scale(Vec3::splat(preview_scale)),
            PreviewBlockIndex { index },
        ));
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

        if game.well.can_place_figure(&candidate) {
            game.active_figure = candidate;
            info!("active_figure position: {:?}", game.active_figure.position);
        } else {
            info!("movement blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyX) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::X);

        if game.well.can_place_figure(&candidate) {
            game.active_figure = candidate;
            info!("rotate X: {:?}", game.active_figure.blocks);
        } else {
            info!("rotation X blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyY) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::Y);

        if game.well.can_place_figure(&candidate) {
            game.active_figure = candidate;
            info!("rotate Y: {:?}", game.active_figure.blocks);
        } else {
            info!("rotation Y blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyZ) {
        let mut candidate = game.active_figure.clone();
        candidate.rotate_90(Axis::Z);

        if game.well.can_place_figure(&candidate) {
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

            if game.well.can_place_figure(&candidate) {
                game.active_figure = candidate;
            } else {
                break;
            }
        }

        let locked_figure = game.active_figure.clone();
        game.well.lock_figure(&locked_figure);

        let cleared_planes = game.well.clear_full_planes();
        if cleared_planes > 0 {
            info!("cleared {} planes", cleared_planes);
        }

        info!("active_figure locked at {:?}", game.active_figure.position);
        info!("occupied cells: {:?}", game.well.occupied);
        info!("cleared planes: {}", cleared_planes);

        if let Some((_block_index, mesh, material)) = figure_blocks.iter().next() {
            for local_block in &locked_figure.blocks {
                let world_block = locked_figure.world_position(*local_block);

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
        }

        game.active_figure = game.next_figure.clone();
        game.next_figure = game.figure_bag.next_figure();

        let can_spawn = game.well.can_place_figure(&game.active_figure);
        if !can_spawn {
            game.game_over = true;
        }
    }
}

fn sync_figure_position(
    game: Res<GameModel>,
    block_visuals: Res<BlockVisualAssets>,
    mut blocks: Query<(
        &FigureBlockIndex,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let active_material = block_visuals.material_for(game.active_figure.color);

    for (block, mut transform, mut material) in &mut blocks {
        let local_block = game.active_figure.blocks[block.index];
        let world_block = game.active_figure.world_position(local_block);

        transform.translation = Vec3::new(
            world_block.x as f32,
            world_block.y as f32,
            world_block.z as f32,
        );

        material.0 = active_material.clone();
    }
}

fn sync_next_figure_preview(
    game: Res<GameModel>,
    block_visuals: Res<BlockVisualAssets>,
    mut blocks: Query<(
        &PreviewBlockIndex,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let preview_material = block_visuals.material_for(game.next_figure.color);
    let preview_scale = 0.7;

    for (block, mut transform, mut material) in &mut blocks {
        let local_block = game.next_figure.blocks[block.index];

        transform.translation = Vec3::new(
            7.0 + local_block.x as f32 * preview_scale,
            3.0 + local_block.y as f32 * preview_scale,
            local_block.z as f32 * preview_scale,
        );

        material.0 = preview_material.clone();
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

fn apply_gravity(
    mut commands: Commands,
    time: Res<Time>,
    mut gravity: ResMut<GravityTimer>,
    mut game: ResMut<GameModel>,
    figure_blocks: Query<(
        &FigureBlockIndex,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if game.game_over {
        return;
    }

    gravity.timer.tick(time.delta());

    if !gravity.timer.just_finished() {
        return;
    }

    let mut candidate = game.active_figure.clone();

    candidate.move_by(Vec3i { x: 0, y: 0, z: 1 });

    if game.well.can_place_figure(&candidate) {
        game.active_figure = candidate;
    } else {
        let locked_figure = game.active_figure.clone();
        game.well.lock_figure(&locked_figure);

        let cleared_planes = game.well.clear_full_planes();
        if cleared_planes > 0 {
            info!("cleared {} planes", cleared_planes);
        }

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

        game.active_figure = game.next_figure.clone();
        game.next_figure = game.figure_bag.next_figure();

        let can_spawn = game.well.can_place_figure(&game.active_figure);

        if !can_spawn {
            game.game_over = true;
            info!("GAME OVER");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure_bag_returns_every_kind_once_before_refill() {
        let mut bag = FigureBag::new();

        let figures = (0..7).map(|_| bag.next_figure()).collect::<Vec<_>>();

        let expected_kinds = [
            FigureKind::I,
            FigureKind::O,
            FigureKind::T,
            FigureKind::L,
            FigureKind::J,
            FigureKind::S,
            FigureKind::Z,
        ];

        for expected_kind in expected_kinds {
            let count = figures
                .iter()
                .filter(|figure| figure.kind == expected_kind)
                .count();

            assert_eq!(count, 1);
        }
    }

    #[test]
    fn clear_full_planes_removes_multiple_planes() {
        let mut well = Well {
            width: 2,
            height: 1,
            depth: 4,
            occupied: vec![
                Vec3i { x: 0, y: 0, z: 1 },
                Vec3i { x: 0, y: 0, z: 2 },
                Vec3i { x: 1, y: 0, z: 2 },
                Vec3i { x: 0, y: 0, z: 3 },
                Vec3i { x: 1, y: 0, z: 3 },
            ],
        };

        let cleared_planes = well.clear_full_planes();

        assert_eq!(cleared_planes, 2);
        assert_eq!(well.occupied.len(), 1);
        assert!(well.is_occupied(Vec3i { x: 0, y: 0, z: 3 }));
    }

    #[test]
    fn clearing_full_plane_removes_it_and_shifts_cells_above() {
        let mut well = Well {
            width: 2,
            height: 2,
            depth: 4,
            occupied: vec![
                Vec3i { x: 0, y: 0, z: 2 },
                Vec3i { x: 1, y: 0, z: 2 },
                Vec3i { x: 0, y: 1, z: 2 },
                Vec3i { x: 1, y: 1, z: 2 },
                Vec3i { x: 0, y: 0, z: 1 },
                Vec3i { x: 1, y: 1, z: 3 },
            ],
        };

        let cleared = well.clear_plane(2);

        assert!(cleared);

        assert!(!well.is_occupied(Vec3i { x: 0, y: 0, z: 1 }));
        assert!(well.is_occupied(Vec3i { x: 0, y: 0, z: 2 }));
        assert!(well.is_occupied(Vec3i { x: 1, y: 1, z: 3 }));

        assert_eq!(well.occupied.len(), 2);
    }

    #[test]
    fn plane_is_full_when_all_its_cells_are_occupied() {
        let well = Well {
            width: 2,
            height: 2,
            depth: 3,
            occupied: vec![
                Vec3i { x: 0, y: 0, z: 2 },
                Vec3i { x: 1, y: 0, z: 2 },
                Vec3i { x: 0, y: 1, z: 2 },
                Vec3i { x: 1, y: 1, z: 2 },
            ],
        };

        assert!(well.is_plane_full(2));
        assert!(!well.is_plane_full(1));
        assert!(!well.is_plane_full(-1));
        assert!(!well.is_plane_full(3));
    }

    #[test]
    fn locking_figure_marks_its_world_cells_as_occupied() {
        let mut well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        };

        let active_figure = Figure {
            kind: FigureKind::I,
            position: Vec3i { x: 2, y: 3, z: 5 },
            blocks: vec![Vec3i { x: 0, y: 0, z: 0 }, Vec3i { x: 1, y: 0, z: 0 }],
            color: FigureColor::Cyan,
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
            kind: FigureKind::I,
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![Vec3i { x: 0, y: 0, z: 0 }, Vec3i { x: 1, y: 0, z: 0 }],
            color: FigureColor::Cyan,
        };

        assert!(!well.can_place_figure(&active_figure));
    }

    #[test]
    fn well_can_place_figure_using_world_positions() {
        let well = Well {
            width: 5,
            height: 5,
            depth: 12,
            occupied: Vec::new(),
        };

        let mut active_figure = Figure {
            kind: FigureKind::I,
            position: Vec3i { x: 3, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
            color: FigureColor::Cyan,
        };

        assert!(well.can_place_figure(&active_figure));

        active_figure.position.x += 1;

        assert!(!well.can_place_figure(&active_figure));
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
            kind: FigureKind::I,
            position: Vec3i { x: 2, y: 3, z: 0 },
            blocks: vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 1, y: 1, z: 0 },
            ],
            color: FigureColor::Cyan,
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
