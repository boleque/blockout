use bevy::prelude::Color as BevyColor;
use bevy::{prelude::*, text::FontSize};
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    Cyan,
    Orange,
    Green,
    Purple,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Material {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    position: Vec3i,
    color: Color,
    material: Material,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Figure {
    kind: FigureKind,
    // world coordinate
    pivot: Vec3i,
    blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Plane {
    blocks: Vec<Option<Block>>,
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
    planes: Vec<Plane>,
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

impl Plane {
    fn empty(blocks_count: usize) -> Self {
        Self {
            blocks: vec![None; blocks_count],
        }
    }

    fn is_full(&self) -> bool {
        self.blocks.iter().all(Option::is_some)
    }
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
            Color::Cyan,
            Color::Orange,
            Color::Green,
            Color::Purple,
            Color::Yellow,
        ];

        let materials = vec![
            Material::Metal,
            Material::Rubber,
            Material::Crystal,
            Material::Neon,
        ];

        self.figures = kinds
            .into_iter()
            .zip(colors.into_iter().cycle())
            .zip(materials.into_iter().cycle())
            .map(|((kind, color), material)| Figure::new(kind, color, material))
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
    fn material_for(&self, color: Color) -> Handle<StandardMaterial> {
        match color {
            Color::Cyan => self.cyan.clone(),
            Color::Orange => self.orange.clone(),
            Color::Green => self.green.clone(),
            Color::Purple => self.purple.clone(),
            Color::Yellow => self.yellow.clone(),
        }
    }
}

impl Well {
    fn new(width: i32, height: i32, depth: i32) -> Self {
        assert!(width > 0);
        assert!(height > 0);
        assert!(depth > 0);

        let plane_block_count = (width * height) as usize;

        Self {
            width,
            height,
            depth,
            planes: vec![Plane::empty(plane_block_count); depth as usize],
        }
    }

    fn contains(&self, position: Vec3i) -> bool {
        position.x >= 0
            && position.x < self.width
            && position.y >= 0
            && position.y < self.height
            && position.z >= 0
            && position.z < self.depth
    }

    fn plane_slot_index(&self, position: Vec3i) -> usize {
        assert!(self.contains(position));

        (position.y * self.width + position.x) as usize
    }

    fn block_at(&self, position: Vec3i) -> Option<Block> {
        if !self.contains(position) {
            return None;
        }

        let plane_index = position.z as usize;
        let slot_index = self.plane_slot_index(position);

        self.planes[plane_index].blocks[slot_index]
    }

    fn is_occupied(&self, position: Vec3i) -> bool {
        self.block_at(position).is_some()
    }

    fn place_block(&mut self, block: Block) {
        assert!(self.contains(block.position));

        let plane_index = block.position.z as usize;
        let slot_index = self.plane_slot_index(block.position);

        let slot = &mut self.planes[plane_index].blocks[slot_index];

        assert!(
            slot.is_none(),
            "cannot place two blocks in the same position"
        );

        *slot = Some(block);
    }

    fn occupied_count(&self) -> usize {
        self.planes
            .iter()
            .flat_map(|plane| plane.blocks.iter())
            .filter(|block| block.is_some())
            .count()
    }

    fn can_place_figure(&self, figure: &Figure) -> bool {
        for block in &figure.blocks {
            if !self.contains(block.position) || self.is_occupied(block.position) {
                return false;
            }
        }

        true
    }

    fn lock_figure(&mut self, figure: &Figure) {
        for block in &figure.blocks {
            self.place_block(*block);
        }
    }

    fn is_plane_full(&self, z: i32) -> bool {
        if z < 0 || z >= self.depth {
            return false;
        }

        self.planes[z as usize].is_full()
    }

    fn update_block_z_coordinates(&mut self) {
        for (z, plane) in self.planes.iter_mut().enumerate() {
            for block in plane.blocks.iter_mut().flatten() {
                block.position.z = z as i32;
            }
        }
    }

    fn clear_plane(&mut self, z: i32) -> bool {
        if !self.is_plane_full(z) {
            return false;
        }

        self.planes.remove(z as usize);

        let plane_block_count = (self.width * self.height) as usize;

        self.planes.insert(0, Plane::empty(plane_block_count));

        self.update_block_z_coordinates();

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
            well: Well::new(5, 5, 12),
            active_figure,
            next_figure,
            show_line: true,
            game_over: false,
            figure_bag,
        }
    }
}

impl Vec3i {
    fn translated(self, delta: Vec3i) -> Self {
        Self {
            x: self.x + delta.x,
            y: self.y + delta.y,
            z: self.z + delta.z,
        }
    }

    fn relative_to(self, origin: Vec3i) -> Self {
        Self {
            x: self.x - origin.x,
            y: self.y - origin.y,
            z: self.z - origin.z,
        }
    }

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
    fn new(kind: FigureKind, color: Color, material: Material) -> Self {
        let local_positions = match kind {
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

        let pivot = Vec3i { x: 2, y: 3, z: 0 };

        let blocks = local_positions
            .into_iter()
            .map(|local_position| Block {
                position: pivot.translated(local_position),
                color,
                material,
            })
            .collect();

        Self {
            kind,
            pivot,
            blocks,
        }
    }

    fn move_by(&mut self, delta: Vec3i) {
        self.pivot = self.pivot.translated(delta);

        for block in &mut self.blocks {
            block.position = block.position.translated(delta);
        }
    }

    fn rotate_90(&mut self, axis: Axis) {
        for block in &mut self.blocks {
            let local_position = block.position.relative_to(self.pivot);
            let rotated_position = local_position.rotated_90(axis);
            block.position = self.pivot.translated(rotated_position);
        }
    }
}

fn make_block_material(base_color: BevyColor) -> StandardMaterial {
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
        .insert_resource(ClearColor(BevyColor::srgb(0.0, 0.0, 0.0)))
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
        TextColor(BevyColor::srgb(1.0, 1.0, 1.0)),
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
        BackgroundColor(BevyColor::srgb(0.0, 1.0, 0.0)),
        DebugLine,
    ));

    commands.spawn((
        Text::new("GAME OVER"),
        TextFont {
            font_size: FontSize::Px(48.0),
            ..default()
        },
        TextColor(BevyColor::srgb(1.0, 0.2, 0.2)),
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
        cyan: materials.add(make_block_material(BevyColor::srgb(0.2, 0.8, 1.0))),
        orange: materials.add(make_block_material(BevyColor::srgb(1.0, 0.4, 0.1))),
        green: materials.add(make_block_material(BevyColor::srgb(0.2, 0.9, 0.3))),
        purple: materials.add(make_block_material(BevyColor::srgb(0.7, 0.2, 1.0))),
        yellow: materials.add(make_block_material(BevyColor::srgb(1.0, 0.85, 0.1))),
    };

    let block_mesh = block_visuals.mesh.clone();

    for (index, block) in game.active_figure.blocks.iter().enumerate() {
        let world_position = block.position;
        let block_material = block_visuals.material_for(block.color);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(block_material),
            Transform::from_xyz(
                world_position.x as f32,
                world_position.y as f32,
                world_position.z as f32,
            ),
            FigureBlockIndex { index },
        ));
    }

    for (index, block) in game.next_figure.blocks.iter().enumerate() {
        let preview_scale = 0.7;
        let local_position = block.position.relative_to(game.next_figure.pivot);
        let preview_material = block_visuals.material_for(block.color);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(preview_material),
            Transform::from_xyz(
                7.0 + local_position.x as f32 * preview_scale,
                3.0 + local_position.y as f32 * preview_scale,
                local_position.z as f32 * preview_scale,
            )
            .with_scale(Vec3::splat(preview_scale)),
            PreviewBlockIndex { index },
        ));
    }

    commands.insert_resource(block_visuals);
}

fn handle_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameModel>,
    block_visuals: Res<BlockVisualAssets>,
    mut line: Query<&mut Visibility, With<DebugLine>>,
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
            info!("active_figure position: {:?}", game.active_figure.pivot);
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

        info!("active_figure locked at {:?}", game.active_figure.pivot);
        info!("occupied cell count: {}", game.well.occupied_count());
        info!("cleared planes: {}", cleared_planes);

        for block in &locked_figure.blocks {
            let world_position = block.position;

            commands.spawn((
                Mesh3d(block_visuals.mesh.clone()),
                MeshMaterial3d(block_visuals.material_for(block.color)),
                Transform::from_xyz(
                    world_position.x as f32,
                    world_position.y as f32,
                    world_position.z as f32,
                ),
                LockedBlock,
            ));
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
    for (block_index, mut transform, mut material) in &mut blocks {
        let block = game.active_figure.blocks[block_index.index];
        let world_position = block.position;

        transform.translation = Vec3::new(
            world_position.x as f32,
            world_position.y as f32,
            world_position.z as f32,
        );

        material.0 = block_visuals.material_for(block.color);
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
    let preview_scale = 0.7;

    for (block_index, mut transform, mut material) in &mut blocks {
        let block = game.next_figure.blocks[block_index.index];
        let local_position = block.position.relative_to(game.next_figure.pivot);

        transform.translation = Vec3::new(
            7.0 + local_position.x as f32 * preview_scale,
            3.0 + local_position.y as f32 * preview_scale,
            local_position.z as f32 * preview_scale,
        );

        material.0 = block_visuals.material_for(block.color);
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
    block_visuals: Res<BlockVisualAssets>,
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

        for block in &locked_figure.blocks {
            let world_position = block.position;

            commands.spawn((
                Mesh3d(block_visuals.mesh.clone()),
                MeshMaterial3d(block_visuals.material_for(block.color)),
                Transform::from_xyz(
                    world_position.x as f32,
                    world_position.y as f32,
                    world_position.z as f32,
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
mod model_tests;
