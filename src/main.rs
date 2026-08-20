use bevy::prelude::Color as BevyColor;
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    text::FontSize,
    ui::widget::ViewportNode,
};
use rand::seq::SliceRandom;
use std::collections::HashMap;

const DESTROYING_BLOCK_LIFETIME_SECONDS: f32 = 0.8;
const SCORE_PER_CLEARED_PLANE: u64 = 100;
const PREVIEW_RENDER_LAYER: usize = 1;
const PREVIEW_CAMERA_DISTANCE: f32 = 4.5;
const PREVIEW_BLOCK_SCALE: f32 = 1.0;
const MIN_LEVEL: u8 = 1;
const MAX_LEVEL: u8 = 10;
const LEVEL_ONE_GRAVITY_SECONDS: f32 = 0.9;
const GRAVITY_SECONDS_PER_LEVEL: f32 = 0.08;
const WELL_WIDTH: i32 = 4;
const WELL_HEIGHT: i32 = 4;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    MainMenu,
    InGame,
    Leaderboard,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RotationDirection {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Color {
    Cyan,
    Orange,
    Green,
    Purple,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Tripod,
    ScrewLeft,
    ScrewRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PitSize {
    Shallow,
    Classic,
    Wide,
}

impl PitSize {
    fn dimensions(self) -> (i32, i32, i32) {
        match self {
            Self::Shallow => (4, 4, 8),
            Self::Classic => (4, 4, 12),
            Self::Wide => (5, 5, 14),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Shallow => "4x4x8",
            Self::Classic => "4x4x12",
            Self::Wide => "5x5x14",
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Shallow => Self::Wide,
            Self::Classic => Self::Shallow,
            Self::Wide => Self::Classic,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Shallow => Self::Classic,
            Self::Classic => Self::Wide,
            Self::Wide => Self::Shallow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockSet {
    Flat,
    Basic3d,
    Extended,
}

impl BlockSet {
    fn label(self) -> &'static str {
        match self {
            Self::Flat => "FLAT",
            Self::Basic3d => "BASIC 3D",
            Self::Extended => "EXTENDED",
        }
    }

    fn figure_kinds(self) -> Vec<FigureKind> {
        let flat = [
            FigureKind::I,
            FigureKind::O,
            FigureKind::T,
            FigureKind::L,
            FigureKind::J,
            FigureKind::S,
            FigureKind::Z,
        ];
        let three_dimensional = [
            FigureKind::Tripod,
            FigureKind::ScrewLeft,
            FigureKind::ScrewRight,
        ];

        match self {
            Self::Flat => flat.to_vec(),
            Self::Basic3d => three_dimensional.to_vec(),
            Self::Extended => flat.into_iter().chain(three_dimensional).collect(),
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Flat => Self::Extended,
            Self::Basic3d => Self::Flat,
            Self::Extended => Self::Basic3d,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Flat => Self::Basic3d,
            Self::Basic3d => Self::Extended,
            Self::Extended => Self::Flat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Vec3i {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Resource)]
struct BlockVisualAssets {
    mesh: Handle<Mesh>,
    materials: HashMap<(Color, Material), Handle<StandardMaterial>>,
    active_materials: HashMap<(Color, Material), Handle<StandardMaterial>>,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
struct GameSettings {
    level: u8,
    pit_size: PitSize,
    block_set: BlockSet,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            level: MIN_LEVEL,
            pit_size: PitSize::Classic,
            block_set: BlockSet::Flat,
        }
    }
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
    block_set: BlockSet,
    spawn_pivot: Vec3i,
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
    score: u64,
    figures_placed: u64,
    show_line: bool,
    game_over: bool,
    figure_bag: FigureBag,
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct DebugLine;

#[derive(Component)]
struct FigureBlockIndex {
    index: usize,
}

#[derive(Component)]
struct LockedBlock;

#[derive(Component)]
struct GameOverOverlay;

#[derive(Component)]
struct FinalScoreText;

#[derive(Component)]
struct PreviewBlockIndex {
    index: usize,
}

#[derive(Resource)]
struct GravityTimer {
    timer: Timer,
}

#[derive(Component)]
struct DestroyingBlock {
    lifetime: Timer,
}

#[derive(Component)]
struct GameCamera;

#[derive(Component)]
struct PreviewCamera;

#[derive(Component)]
struct UiCamera;

#[derive(Component)]
struct GameScreenRoot;

#[derive(Component)]
struct GameViewportArea;

#[derive(Component)]
struct FiguresPlacedText;

#[derive(Component)]
struct PitDepthCell {
    z_index: usize,
}

#[derive(Component)]
struct RestartButton;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct MainMenuButton;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct LeaderboardButton;

#[derive(Component)]
struct SettingsButton;

#[derive(Component)]
struct GameSettingsRoot;

#[derive(Component)]
struct SettingsBackButton;

#[derive(Component, Clone, Copy)]
enum LevelAdjustment {
    Decrease,
    Increase,
}

#[derive(Component)]
struct SettingsLevelText;

#[derive(Debug, Clone, Copy)]
enum SelectionDirection {
    Previous,
    Next,
}

#[derive(Component)]
struct PitSizeAdjustment {
    direction: SelectionDirection,
}

#[derive(Component)]
struct BlockSetAdjustment {
    direction: SelectionDirection,
}

#[derive(Component)]
struct SettingsPitSizeText;

#[derive(Component)]
struct SettingsBlockSetText;

#[derive(Component)]
struct QuitButton;

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
    fn new(block_set: BlockSet, pit_size: PitSize) -> Self {
        let (width, height, _) = pit_size.dimensions();
        let mut bag = Self {
            figures: Vec::new(),
            block_set,
            spawn_pivot: Vec3i {
                x: width / 2 - 1,
                y: height / 2 - 1,
                z: 0,
            },
        };
        bag.refill();

        bag
    }

    fn refill(&mut self) {
        let kinds = self.block_set.figure_kinds();

        let colors = vec![
            Color::Orange,
            Color::Cyan,
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
            .map(|((kind, color), material)| {
                Figure::new_at(kind, color, material, self.spawn_pivot)
            })
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
    fn material_for(&self, block: Block) -> Handle<StandardMaterial> {
        self.materials
            .get(&(block.color, block.material))
            .expect("every block appearance must have a visual material")
            .clone()
    }

    fn active_material_for(&self, block: Block) -> Handle<StandardMaterial> {
        self.active_materials
            .get(&(block.color, block.material))
            .expect("every active block appearance must have a translucent material")
            .clone()
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

    fn clear_plane(&mut self, z: i32) -> Option<Plane> {
        if !self.is_plane_full(z) {
            return None;
        }

        let removed_plane = self.planes.remove(z as usize);

        let plane_block_count = (self.width * self.height) as usize;

        self.planes.insert(0, Plane::empty(plane_block_count));

        self.update_block_z_coordinates();

        Some(removed_plane)
    }

    fn clear_full_planes(&mut self) -> Vec<Plane> {
        let mut cleared_planes = Vec::new();
        let mut z = self.depth - 1;

        while z >= 0 {
            if let Some(plane) = self.clear_plane(z) {
                cleared_planes.push(plane);
            } else {
                z -= 1;
            }
        }

        cleared_planes
    }
}

impl GameModel {
    fn new(settings: GameSettings) -> Self {
        let mut figure_bag = FigureBag::new(settings.block_set, settings.pit_size);
        let active_figure = figure_bag.next_figure();
        let next_figure = figure_bag.next_figure();
        let (well_width, well_height, well_depth) = settings.pit_size.dimensions();

        Self {
            well: Well::new(well_width, well_height, well_depth),
            active_figure: active_figure,
            next_figure: next_figure,
            show_line: true,
            game_over: false,
            figure_bag: figure_bag,
            score: 0,
            figures_placed: 0,
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
        Self::new_at(
            kind,
            color,
            material,
            Vec3i {
                x: WELL_WIDTH / 2 - 1,
                y: WELL_HEIGHT / 2 - 1,
                z: 0,
            },
        )
    }

    fn new_at(kind: FigureKind, color: Color, material: Material, pivot: Vec3i) -> Self {
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
            FigureKind::Tripod => vec![
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
                Vec3i { x: 0, y: 0, z: 1 },
            ],
            FigureKind::ScrewLeft => vec![
                Vec3i { x: -1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
                Vec3i { x: 0, y: 1, z: 1 },
            ],
            FigureKind::ScrewRight => vec![
                Vec3i { x: 1, y: 0, z: 0 },
                Vec3i { x: 0, y: 0, z: 0 },
                Vec3i { x: 0, y: 1, z: 0 },
                Vec3i { x: 0, y: 1, z: 1 },
            ],
        };

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

fn sorted_orientation(mut positions: Vec<Vec3i>) -> Vec<Vec3i> {
    positions.sort_by_key(|position| (position.x, position.y, position.z));
    positions
}

fn orientation_signature(figure: &Figure) -> Vec<Vec3i> {
    sorted_orientation(
        figure
            .blocks
            .iter()
            .map(|block| block.position.relative_to(figure.pivot))
            .collect(),
    )
}

fn normalized_orientation(positions: &[Vec3i]) -> Vec<Vec3i> {
    let min_x = positions
        .iter()
        .map(|position| position.x)
        .min()
        .unwrap_or(0);
    let min_y = positions
        .iter()
        .map(|position| position.y)
        .min()
        .unwrap_or(0);
    let min_z = positions
        .iter()
        .map(|position| position.z)
        .min()
        .unwrap_or(0);

    sorted_orientation(
        positions
            .iter()
            .map(|position| Vec3i {
                x: position.x - min_x,
                y: position.y - min_y,
                z: position.z - min_z,
            })
            .collect(),
    )
}

fn unique_figure_orientations(kind: FigureKind) -> Vec<Vec<Vec3i>> {
    let initial_figure = Figure::new(kind, Color::Cyan, Material::Metal);
    let mut orientations = vec![orientation_signature(&initial_figure)];
    let mut next_orientation_index = 0;

    while next_orientation_index < orientations.len() {
        let current = orientations[next_orientation_index].clone();

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let rotated = sorted_orientation(
                current
                    .iter()
                    .map(|position| position.rotated_90(axis))
                    .collect(),
            );

            let rotated_shape = normalized_orientation(&rotated);
            let already_known = orientations
                .iter()
                .any(|orientation| normalized_orientation(orientation) == rotated_shape);

            if !already_known {
                orientations.push(rotated);
            }
        }

        next_orientation_index += 1;
    }

    orientations
}

fn figure_with_entrance_kick(well: &Well, mut candidate: Figure) -> Option<Figure> {
    let min_z = candidate
        .blocks
        .iter()
        .map(|block| block.position.z)
        .min()?;

    if min_z < 0 {
        candidate.move_by(Vec3i {
            x: 0,
            y: 0,
            z: -min_z,
        });
    }

    well.can_place_figure(&candidate).then_some(candidate)
}

fn figure_with_next_orientation(well: &Well, figure: &Figure) -> Option<Figure> {
    let orientations = unique_figure_orientations(figure.kind);
    let current = orientation_signature(figure);
    let current_shape = normalized_orientation(&current);
    let current_index = orientations
        .iter()
        .position(|orientation| normalized_orientation(orientation) == current_shape)
        .unwrap_or(0);

    for offset in 1..orientations.len() {
        let orientation = &orientations[(current_index + offset) % orientations.len()];
        let mut candidate = figure.clone();

        for (block, local_position) in candidate.blocks.iter_mut().zip(orientation) {
            block.position = candidate.pivot.translated(*local_position);
        }

        if let Some(candidate) = figure_with_entrance_kick(well, candidate) {
            return Some(candidate);
        }
    }

    None
}

fn rotated_figure_with_entrance_kick(
    well: &Well,
    figure: &Figure,
    axis: Axis,
    direction: RotationDirection,
) -> Option<Figure> {
    let mut candidate = figure.clone();
    let quarter_turns = match direction {
        RotationDirection::Positive => 1,
        RotationDirection::Negative => 3,
    };

    for _ in 0..quarter_turns {
        candidate.rotate_90(axis);
    }

    figure_with_entrance_kick(well, candidate)
}

fn gravity_seconds_for_level(level: u8) -> f32 {
    let level = level.clamp(MIN_LEVEL, MAX_LEVEL);
    LEVEL_ONE_GRAVITY_SECONDS - (level - MIN_LEVEL) as f32 * GRAVITY_SECONDS_PER_LEVEL
}

fn block_visual_color(color: Color) -> BevyColor {
    match color {
        Color::Cyan => BevyColor::srgb(0.08, 0.68, 0.72),
        Color::Orange => BevyColor::srgb(0.9, 0.12, 0.04),
        Color::Green => BevyColor::srgb(0.3, 0.78, 0.16),
        Color::Purple => BevyColor::srgb(0.7, 0.12, 0.72),
        Color::Yellow => BevyColor::srgb(0.95, 0.68, 0.06),
    }
}

fn make_block_material(base_color: BevyColor, _material: Material) -> StandardMaterial {
    StandardMaterial {
        base_color,
        metallic: 0.0,
        perceptual_roughness: 0.72,
        ..default()
    }
}

fn make_active_block_material(base_color: BevyColor, material: Material) -> StandardMaterial {
    StandardMaterial {
        base_color: base_color.with_alpha(0.42),
        alpha_mode: AlphaMode::Blend,
        ..make_block_material(base_color, material)
    }
}

fn logical_position_to_bevy_translation(position: Vec3i) -> Vec3 {
    Vec3::new(position.x as f32, position.y as f32, position.z as f32)
}

fn game_camera_z_for_well(well: &Well) -> f32 {
    let entrance_z = -0.5;
    let largest_entrance_dimension = well.width.max(well.height) as f32;

    entrance_z - largest_entrance_dimension * 2.0
}

fn preview_figure_center(figure: &Figure) -> Vec3 {
    let mut local_positions = figure.blocks.iter().map(|block| {
        logical_position_to_bevy_translation(block.position.relative_to(figure.pivot))
    });
    let first_position = local_positions.next().unwrap_or(Vec3::ZERO);
    let (min, max) = local_positions
        .fold((first_position, first_position), |(min, max), position| {
            (min.min(position), max.max(position))
        });

    (min + max) * 0.5
}

fn preview_block_translation(block: Block, figure: &Figure, scale: f32) -> Vec3 {
    let local_position =
        logical_position_to_bevy_translation(block.position.relative_to(figure.pivot));

    (local_position - preview_figure_center(figure)) * scale
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Blockout".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .insert_resource(GameSettings::default())
        .insert_resource(GameModel::new(GameSettings::default()))
        .insert_resource(ClearColor(BevyColor::srgb(0.0, 0.0, 0.0)))
        .insert_resource(GravityTimer {
            timer: Timer::from_seconds(gravity_seconds_for_level(MIN_LEVEL), TimerMode::Repeating),
        })
        .add_systems(Startup, setup_main_ui_camera)
        .add_systems(OnEnter(AppState::MainMenu), setup_game_main_menu)
        .add_systems(OnEnter(AppState::Settings), setup_game_settings)
        .add_systems(OnEnter(AppState::InGame), setup_game)
        .add_systems(
            Update,
            handle_play_button.run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(
            Update,
            handle_settings_button.run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(
            Update,
            (
                handle_level_adjustment_buttons,
                handle_pit_size_adjustment_buttons,
                handle_block_set_adjustment_buttons,
                sync_settings_level_text,
                sync_settings_pit_size_text,
                sync_settings_block_set_text,
                handle_settings_back_button,
            )
                .chain()
                .run_if(in_state(AppState::Settings)),
        )
        .add_systems(
            Update,
            handle_quit_button.run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(
            Update,
            handle_main_menu_button.run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                handle_input,
                apply_gravity,
                sync_score_text,
                sync_figures_placed_text,
                sync_figure_position,
                sync_next_figure_preview,
                sync_pit_depth_meter,
                sync_game_over_text,
                draw_well,
                animate_destroying_blocks,
                handle_restart_button,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .run();
}

fn setup_main_ui_camera(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera, UiCamera));
}

fn setup_game(
    mut commands: Commands,
    mut game: ResMut<GameModel>,
    game_settings: Res<GameSettings>,
    mut gravity: ResMut<GravityTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    *game = GameModel::new(*game_settings);
    gravity.timer = Timer::from_seconds(
        gravity_seconds_for_level(game_settings.level),
        TimerMode::Repeating,
    );

    // calculate well center
    let well_center_x = (game.well.width - 1) as f32 * 0.5;
    let well_center_y = (game.well.height - 1) as f32 * 0.5;
    let well_center_z = (game.well.depth - 1) as f32 * 0.5;

    let mut game_viewport_image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );

    game_viewport_image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let game_viewport_image_handle = images.add(game_viewport_image);

    let game_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(game_viewport_image_handle.clone().into()),
            Transform::from_xyz(
                well_center_x,
                well_center_y,
                game_camera_z_for_well(&game.well),
            )
            .looking_at(
                Vec3::new(well_center_x, well_center_y, well_center_z),
                Vec3::Y,
            ),
            GameCamera,
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    let mut preview_viewport_image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );

    preview_viewport_image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let preview_viewport_image_handle = images.add(preview_viewport_image);

    let preview_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -1,
                clear_color: BevyColor::BLACK.into(),
                ..default()
            },
            RenderTarget::Image(preview_viewport_image_handle.into()),
            Transform::from_xyz(0.0, 0.0, -PREVIEW_CAMERA_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
            RenderLayers::layer(PREVIEW_RENDER_LAYER),
            PreviewCamera,
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.0)),
            GameScreenRoot,
            DespawnOnExit(AppState::InGame),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: percent(8.0),
                    height: percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(8.0)),
                    border: UiRect::right(px(3.0)),
                    ..default()
                },
                BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.015)),
                BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.65)),
            ))
            .with_children(|left_panel| {
                left_panel
                    .spawn((
                        Node {
                            width: percent(72.0),
                            height: percent(92.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::vertical(px(3.0)),
                            border: UiRect::horizontal(px(3.0)),
                            ..default()
                        },
                        BackgroundColor(BevyColor::BLACK),
                        BorderColor::all(BevyColor::srgb(0.12, 0.62, 0.16)),
                    ))
                    .with_children(|meter| {
                        for z_index in 0..game.well.depth as usize {
                            meter.spawn((
                                Node {
                                    width: percent(100.0),
                                    flex_grow: 1.0,
                                    border: UiRect::bottom(px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.04, 0.2, 0.06)),
                                PitDepthCell { z_index },
                            ));
                        }
                    });
            });

            root.spawn((
                Node {
                    flex_grow: 1.0,
                    height: percent(100.0),
                    position_type: PositionType::Relative,
                    border: UiRect::horizontal(px(2.0)),
                    ..default()
                },
                BackgroundColor(BevyColor::BLACK),
                BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.4)),
                GameViewportArea,
            ))
            .with_children(|viewport_area| {
                viewport_area.spawn((
                    Node {
                        width: percent(100.0),
                        height: percent(100.0),
                        ..default()
                    },
                    ViewportNode::new(game_camera),
                ));

                viewport_area
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0.0),
                            right: px(0.0),
                            top: px(0.0),
                            bottom: px(0.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BevyColor::srgba(0.0, 0.0, 0.0, 0.72)),
                        Visibility::Hidden,
                        GlobalZIndex(10),
                        GameOverOverlay,
                    ))
                    .with_children(|overlay| {
                        overlay
                            .spawn((
                                Node {
                                    width: percent(80.0),
                                    max_width: px(420.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(px(32.0)),
                                    row_gap: px(20.0),
                                    border: UiRect::all(px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::srgb(0.01, 0.02, 0.04)),
                                BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
                            ))
                            .with_children(|modal| {
                                modal.spawn((
                                    Text::new("GAME OVER"),
                                    TextFont {
                                        font_size: FontSize::Px(42.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(1.0, 0.15, 0.15)),
                                ));

                                modal.spawn((
                                    Text::new("FINAL SCORE"),
                                    TextFont {
                                        font_size: FontSize::Px(18.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                ));

                                modal.spawn((
                                    Text::new(format!("{:06}", game.score)),
                                    TextFont {
                                        font_size: FontSize::Px(34.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                                    FinalScoreText,
                                ));

                                modal
                                    .spawn((
                                        Button,
                                        Node {
                                            width: percent(100.0),
                                            height: px(56.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                        BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                        RestartButton,
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new("RESTART"),
                                            TextFont {
                                                font_size: FontSize::Px(22.0),
                                                ..default()
                                            },
                                            TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                                        ));
                                    });

                                modal
                                    .spawn((
                                        Button,
                                        Node {
                                            width: percent(100.0),
                                            height: px(56.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                        BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                        MainMenuButton,
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new("MAIN MENU"),
                                            TextFont {
                                                font_size: FontSize::Px(22.0),
                                                ..default()
                                            },
                                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                        ));
                                    });
                            });
                    });
            });
            root.spawn((
                Node {
                    width: percent(19.0),
                    height: percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(12.0)),
                    row_gap: px(8.0),
                    border: UiRect::left(px(3.0)),
                    ..default()
                },
                BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.015)),
                BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.65)),
            ))
            .with_children(|right_panel| {
                // LOGO
                right_panel
                    .spawn((
                        Node {
                            width: percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::bottom(px(10.0)),
                            border: UiRect::bottom(px(3.0)),
                            ..default()
                        },
                        BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.65)),
                    ))
                    .with_children(|logo| {
                        logo.spawn((
                            Text::new("BLOCK"),
                            TextFont {
                                font_size: FontSize::Px(34.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(1.0, 0.15, 0.15)),
                        ));

                        logo.spawn((
                            Text::new("OUT"),
                            TextFont {
                                font_size: FontSize::Px(34.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.1, 0.45, 1.0)),
                        ));
                    });

                // LEVEL
                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|level_section| {
                        level_section.spawn((
                            Text::new("LEVEL"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        level_section
                            .spawn((
                                Node {
                                    width: percent(100.0),
                                    height: px(38.0),
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(px(10.0)),
                                    border: UiRect::all(px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            ))
                            .with_children(|value| {
                                value.spawn((
                                    Text::new(game_settings.level.to_string()),
                                    TextFont {
                                        font_size: FontSize::Px(25.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.92, 0.58, 0.16)),
                                ));
                            });
                    });

                // NEXT BLOCK
                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|next_block_section| {
                        next_block_section.spawn((
                            Text::new("NEXT"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        next_block_section.spawn((
                            Node {
                                width: percent(100.0),
                                height: px(104.0),
                                border: UiRect::all(px(3.0)),
                                ..default()
                            },
                            BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            BackgroundColor(BevyColor::BLACK),
                            ViewportNode::new(preview_camera),
                        ));
                    });

                // SCORE
                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|score_section| {
                        score_section.spawn((
                            Text::new("SCORE"),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        score_section
                            .spawn((
                                Node {
                                    width: percent(100.0),
                                    height: px(42.0),
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(px(10.0)),
                                    border: UiRect::all(px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            ))
                            .with_children(|value| {
                                value.spawn((
                                    Text::new(format!("{:06}", game.score)),
                                    TextFont {
                                        font_size: FontSize::Px(27.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.25, 0.9, 0.22)),
                                    ScoreText,
                                ));
                            });
                    });

                // CUBES PLAYED
                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|cubes_section| {
                        cubes_section.spawn((
                            Text::new("CUBES PLAYED"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        cubes_section
                            .spawn((
                                Node {
                                    width: percent(100.0),
                                    height: px(42.0),
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(px(10.0)),
                                    border: UiRect::all(px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            ))
                            .with_children(|value| {
                                value.spawn((
                                    Text::new(format!("{:03}", game.figures_placed)),
                                    TextFont {
                                        font_size: FontSize::Px(27.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.25, 0.9, 0.22)),
                                    FiguresPlacedText,
                                ));
                            });
                    });

                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|pit_section| {
                        pit_section.spawn((
                            Text::new("PIT"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        pit_section
                            .spawn((
                                Node {
                                    width: percent(100.0),
                                    height: px(38.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            ))
                            .with_children(|value| {
                                value.spawn((
                                    Text::new(format!(
                                        "{}x{}x{}",
                                        game.well.width, game.well.height, game.well.depth
                                    )),
                                    TextFont {
                                        font_size: FontSize::Px(22.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.92, 0.58, 0.16)),
                                ));
                            });
                    });

                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(4.0),
                        ..default()
                    })
                    .with_children(|set_section| {
                        set_section.spawn((
                            Text::new("BLOCK SET"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
                        ));
                        set_section
                            .spawn((
                                Node {
                                    width: percent(100.0),
                                    height: px(38.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(BevyColor::BLACK),
                                BorderColor::all(BevyColor::srgb(0.02, 0.06, 0.55)),
                            ))
                            .with_children(|value| {
                                value.spawn((
                                    Text::new(game_settings.block_set.label()),
                                    TextFont {
                                        font_size: FontSize::Px(17.0),
                                        ..default()
                                    },
                                    TextColor(BevyColor::srgb(0.92, 0.58, 0.16)),
                                ));
                            });
                    });
            });
        });

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, -4.0),
        RenderLayers::layer(0).with(PREVIEW_RENDER_LAYER),
        DespawnOnExit(AppState::InGame),
    ));

    let block_colors = [
        Color::Cyan,
        Color::Orange,
        Color::Green,
        Color::Purple,
        Color::Yellow,
    ];
    let block_material_kinds = [
        Material::Metal,
        Material::Rubber,
        Material::Crystal,
        Material::Neon,
    ];
    let mut block_materials = HashMap::new();
    let mut active_block_materials = HashMap::new();

    for color in block_colors {
        for material in block_material_kinds {
            let base_color = block_visual_color(color);
            let visual_material = materials.add(make_block_material(base_color, material));
            let active_visual_material =
                materials.add(make_active_block_material(base_color, material));
            block_materials.insert((color, material), visual_material);
            active_block_materials.insert((color, material), active_visual_material);
        }
    }

    let block_visuals = BlockVisualAssets {
        mesh: meshes.add(Cuboid::new(0.9, 0.9, 0.9)),
        materials: block_materials,
        active_materials: active_block_materials,
    };

    let block_mesh = block_visuals.mesh.clone();

    for (index, block) in game.active_figure.blocks.iter().enumerate() {
        let world_position = block.position;
        let active_block_material = block_visuals.active_material_for(*block);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(active_block_material),
            Transform::from_translation(logical_position_to_bevy_translation(world_position)),
            FigureBlockIndex { index },
            DespawnOnExit(AppState::InGame),
        ));
    }

    for (index, block) in game.next_figure.blocks.iter().enumerate() {
        let preview_translation =
            preview_block_translation(*block, &game.next_figure, PREVIEW_BLOCK_SCALE);
        let preview_material = block_visuals.material_for(*block);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(preview_material),
            Transform::from_translation(preview_translation)
                .with_scale(Vec3::splat(PREVIEW_BLOCK_SCALE)),
            RenderLayers::layer(PREVIEW_RENDER_LAYER),
            PreviewBlockIndex { index },
            DespawnOnExit(AppState::InGame),
        ));
    }

    commands.insert_resource(block_visuals);
}

fn rebuild_locked_block_visuals(
    commands: &mut Commands,
    well: &Well,
    block_visuals: &BlockVisualAssets,
    locked_blocks: &Query<Entity, With<LockedBlock>>,
) {
    for entity in locked_blocks {
        commands.entity(entity).despawn();
    }

    for plane in &well.planes {
        for block in plane.blocks.iter().flatten() {
            commands.spawn((
                Mesh3d(block_visuals.mesh.clone()),
                MeshMaterial3d(block_visuals.material_for(*block)),
                Transform::from_translation(logical_position_to_bevy_translation(block.position)),
                LockedBlock,
                DespawnOnExit(AppState::InGame),
            ));
        }
    }
}

fn handle_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameModel>,
    block_visuals: Res<BlockVisualAssets>,
    mut line: Query<&mut Visibility, With<DebugLine>>,
    locked_blocks: Query<Entity, With<LockedBlock>>,
) {
    if game.game_over {
        return;
    }

    let mut delta = Vec3i { x: 0, y: 0, z: 0 };

    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        delta.x += 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        delta.x -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        delta.y -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        delta.y += 1;
    }

    if delta.x != 0 || delta.y != 0 {
        let mut candidate = game.active_figure.clone();
        candidate.move_by(delta);

        if game.well.can_place_figure(&candidate) {
            game.active_figure = candidate;
            info!("active_figure position: {:?}", game.active_figure.pivot);
        } else {
            info!("movement blocked by well bounds");
        }
    }

    if keyboard.just_pressed(KeyCode::KeyR) {
        if let Some(candidate) = figure_with_next_orientation(&game.well, &game.active_figure) {
            game.active_figure = candidate;
            info!("cycled to the next 3D orientation");
        } else {
            info!("all alternative orientations are blocked");
        }
    }

    let rotation = if keyboard.just_pressed(KeyCode::KeyQ) {
        Some((Axis::X, RotationDirection::Positive))
    } else if keyboard.just_pressed(KeyCode::KeyA) {
        Some((Axis::X, RotationDirection::Negative))
    } else if keyboard.just_pressed(KeyCode::KeyW) {
        Some((Axis::Y, RotationDirection::Positive))
    } else if keyboard.just_pressed(KeyCode::KeyS) {
        Some((Axis::Y, RotationDirection::Negative))
    } else if keyboard.just_pressed(KeyCode::KeyE) {
        Some((Axis::Z, RotationDirection::Positive))
    } else if keyboard.just_pressed(KeyCode::KeyD) {
        Some((Axis::Z, RotationDirection::Negative))
    } else {
        None
    };

    if let Some((axis, direction)) = rotation {
        if let Some(candidate) =
            rotated_figure_with_entrance_kick(&game.well, &game.active_figure, axis, direction)
        {
            game.active_figure = candidate;
            info!(
                "rotate {:?} {:?}: {:?}",
                axis, direction, game.active_figure.blocks
            );
        } else {
            info!(
                "rotation {:?} {:?} blocked by well bounds or occupied cells",
                axis, direction
            );
        }
    }

    if keyboard.just_pressed(KeyCode::KeyG) {
        game.show_line = !game.show_line;

        for mut visibility in &mut line {
            *visibility = if game.show_line {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
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
        game.figures_placed += 1;

        let cleared_planes: Vec<Plane> = game.well.clear_full_planes();
        let earned_score = score_for_cleared_planes(cleared_planes.len());
        game.score += earned_score;
        if earned_score > 0 {
            info!(
                "earned score: {}, total score: {}",
                earned_score, game.score
            );
        }

        for plane in &cleared_planes {
            for block in plane.blocks.iter().flatten() {
                let neon_block = Block {
                    color: Color::Yellow,
                    material: Material::Neon,
                    ..*block
                };
                commands.spawn((
                    Mesh3d(block_visuals.mesh.clone()),
                    MeshMaterial3d(block_visuals.material_for(neon_block)),
                    Transform::from_translation(logical_position_to_bevy_translation(
                        block.position,
                    )),
                    DestroyingBlock {
                        lifetime: Timer::from_seconds(
                            DESTROYING_BLOCK_LIFETIME_SECONDS,
                            TimerMode::Once,
                        ),
                    },
                    DespawnOnExit(AppState::InGame),
                ));
            }
        }

        if cleared_planes.len() > 0 {
            info!("cleared {} planes", cleared_planes.len());
        }

        rebuild_locked_block_visuals(&mut commands, &game.well, &block_visuals, &locked_blocks);

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

        transform.translation = logical_position_to_bevy_translation(block.position);

        material.0 = block_visuals.active_material_for(block);
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
    for (block_index, mut transform, mut material) in &mut blocks {
        let block = game.next_figure.blocks[block_index.index];

        transform.translation =
            preview_block_translation(block, &game.next_figure, PREVIEW_BLOCK_SCALE);

        material.0 = block_visuals.material_for(block);
    }
}

fn sync_pit_depth_meter(
    game: Res<GameModel>,
    mut cells: Query<(&PitDepthCell, &mut BackgroundColor)>,
) {
    for (cell, mut background) in &mut cells {
        let active_color = game
            .active_figure
            .blocks
            .iter()
            .find(|block| block.position.z == cell.z_index as i32)
            .map(|block| block.color);
        let locked_color = game.well.planes[cell.z_index]
            .blocks
            .iter()
            .flatten()
            .next()
            .map(|block| block.color);

        background.0 = active_color
            .or(locked_color)
            .map(block_visual_color)
            .unwrap_or(BevyColor::BLACK);
    }
}

fn sync_score_text(game: Res<GameModel>, mut score_texts: Query<&mut Text, With<ScoreText>>) {
    if !game.is_changed() {
        return;
    }

    for mut text in &mut score_texts {
        text.0 = format!("{:06}", game.score);
    }
}

fn sync_figures_placed_text(
    game: Res<GameModel>,
    mut figures_placed_texts: Query<&mut Text, With<FiguresPlacedText>>,
) {
    if !game.is_changed() {
        return;
    }

    for mut text in &mut figures_placed_texts {
        text.0 = format!("{:03}", game.figures_placed);
    }
}

fn sync_game_over_text(
    game: Res<GameModel>,
    mut overlays: Query<&mut Visibility, With<GameOverOverlay>>,
    mut final_score_texts: Query<&mut Text, With<FinalScoreText>>,
) {
    if !game.is_changed() {
        return;
    }

    for mut visibility in &mut overlays {
        *visibility = if game.game_over {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if game.game_over {
        for mut text in &mut final_score_texts {
            text.0 = format!("{:06}", game.score);
        }
    }
}

fn draw_well(mut gizmos: Gizmos, game: Res<GameModel>) {
    let min_x = -0.5;
    let max_x = game.well.width as f32 - 0.5;
    let min_y = -0.5;
    let max_y = game.well.height as f32 - 0.5;
    let entrance_z = -0.5;
    let bottom_z = game.well.depth as f32 - 0.5;
    let wall_guide_color = BevyColor::srgba(0.12, 0.52, 0.14, 0.42);
    let entrance_color = BevyColor::srgb(0.72, 0.8, 0.72);
    let bottom_color = BevyColor::srgba(0.18, 0.68, 0.16, 0.88);

    for z_index in 0..=game.well.depth {
        let z = z_index as f32 - 0.5;
        let color = if z_index == 0 {
            entrance_color
        } else if z_index == game.well.depth {
            bottom_color
        } else {
            let depth_fraction = z_index as f32 / game.well.depth as f32;
            let alpha = 0.5 - depth_fraction * 0.24;
            BevyColor::srgba(0.12, 0.58, 0.14, alpha)
        };

        gizmos.line(
            Vec3::new(min_x, min_y, z),
            Vec3::new(max_x, min_y, z),
            color,
        );
        gizmos.line(
            Vec3::new(max_x, min_y, z),
            Vec3::new(max_x, max_y, z),
            color,
        );
        gizmos.line(
            Vec3::new(max_x, max_y, z),
            Vec3::new(min_x, max_y, z),
            color,
        );
        gizmos.line(
            Vec3::new(min_x, max_y, z),
            Vec3::new(min_x, min_y, z),
            color,
        );
    }

    for x_index in 0..=game.well.width {
        let x = x_index as f32 - 0.5;

        gizmos.line(
            Vec3::new(x, min_y, entrance_z),
            Vec3::new(x, min_y, bottom_z),
            wall_guide_color,
        );
        gizmos.line(
            Vec3::new(x, max_y, entrance_z),
            Vec3::new(x, max_y, bottom_z),
            wall_guide_color,
        );
        gizmos.line(
            Vec3::new(x, min_y, bottom_z),
            Vec3::new(x, max_y, bottom_z),
            bottom_color,
        );
    }

    for y_index in 0..=game.well.height {
        let y = y_index as f32 - 0.5;

        gizmos.line(
            Vec3::new(min_x, y, entrance_z),
            Vec3::new(min_x, y, bottom_z),
            wall_guide_color,
        );
        gizmos.line(
            Vec3::new(max_x, y, entrance_z),
            Vec3::new(max_x, y, bottom_z),
            wall_guide_color,
        );
        gizmos.line(
            Vec3::new(min_x, y, bottom_z),
            Vec3::new(max_x, y, bottom_z),
            bottom_color,
        );
    }
}

fn apply_gravity(
    mut commands: Commands,
    time: Res<Time>,
    mut gravity: ResMut<GravityTimer>,
    mut game: ResMut<GameModel>,
    block_visuals: Res<BlockVisualAssets>,
    locked_blocks: Query<Entity, With<LockedBlock>>,
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
        game.figures_placed += 1;

        let cleared_planes = game.well.clear_full_planes();
        let earned_score = score_for_cleared_planes(cleared_planes.len());
        game.score += earned_score;
        if earned_score > 0 {
            info!(
                "earned score: {}, total score: {}",
                earned_score, game.score
            );
        }

        for plane in &cleared_planes {
            for block in plane.blocks.iter().flatten() {
                let neon_block = Block {
                    color: Color::Yellow,
                    material: Material::Neon,
                    ..*block
                };

                commands.spawn((
                    Mesh3d(block_visuals.mesh.clone()),
                    MeshMaterial3d(block_visuals.material_for(neon_block)),
                    Transform::from_translation(logical_position_to_bevy_translation(
                        block.position,
                    )),
                    DestroyingBlock {
                        lifetime: Timer::from_seconds(
                            DESTROYING_BLOCK_LIFETIME_SECONDS,
                            TimerMode::Once,
                        ),
                    },
                    DespawnOnExit(AppState::InGame),
                ));
            }
        }

        rebuild_locked_block_visuals(&mut commands, &game.well, &block_visuals, &locked_blocks);

        game.active_figure = game.next_figure.clone();
        game.next_figure = game.figure_bag.next_figure();

        let can_spawn = game.well.can_place_figure(&game.active_figure);

        if !can_spawn {
            game.game_over = true;
        }
    }
}

fn animate_destroying_blocks(
    mut commands: Commands,
    time: Res<Time>,
    mut destroying_blocks: Query<(Entity, &mut Transform, &mut DestroyingBlock)>,
) {
    for (entity, mut transform, mut destroying_block) in &mut destroying_blocks {
        destroying_block.lifetime.tick(time.delta());

        let scale = destroying_block.lifetime.fraction_remaining();
        transform.scale = Vec3::splat(scale);

        if destroying_block.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_play_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::InGame);
        }
    }
}

fn handle_settings_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<SettingsButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Settings);
        }
    }
}

fn handle_level_adjustment_buttons(
    buttons: Query<(&Interaction, &LevelAdjustment), Changed<Interaction>>,
    mut game_settings: ResMut<GameSettings>,
) {
    for (interaction, adjustment) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        game_settings.level = match adjustment {
            LevelAdjustment::Decrease => game_settings.level.saturating_sub(1).max(MIN_LEVEL),
            LevelAdjustment::Increase => game_settings.level.saturating_add(1).min(MAX_LEVEL),
        };
    }
}

fn handle_pit_size_adjustment_buttons(
    buttons: Query<(&Interaction, &PitSizeAdjustment), Changed<Interaction>>,
    mut game_settings: ResMut<GameSettings>,
) {
    for (interaction, adjustment) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        game_settings.pit_size = match adjustment.direction {
            SelectionDirection::Previous => game_settings.pit_size.previous(),
            SelectionDirection::Next => game_settings.pit_size.next(),
        };
    }
}

fn handle_block_set_adjustment_buttons(
    buttons: Query<(&Interaction, &BlockSetAdjustment), Changed<Interaction>>,
    mut game_settings: ResMut<GameSettings>,
) {
    for (interaction, adjustment) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        game_settings.block_set = match adjustment.direction {
            SelectionDirection::Previous => game_settings.block_set.previous(),
            SelectionDirection::Next => game_settings.block_set.next(),
        };
    }
}

fn sync_settings_level_text(
    game_settings: Res<GameSettings>,
    mut level_texts: Query<&mut Text, With<SettingsLevelText>>,
) {
    if !game_settings.is_changed() {
        return;
    }

    for mut text in &mut level_texts {
        text.0 = game_settings.level.to_string();
    }
}

fn sync_settings_pit_size_text(
    game_settings: Res<GameSettings>,
    mut pit_size_texts: Query<&mut Text, With<SettingsPitSizeText>>,
) {
    if !game_settings.is_changed() {
        return;
    }

    for mut text in &mut pit_size_texts {
        text.0 = game_settings.pit_size.label().to_owned();
    }
}

fn sync_settings_block_set_text(
    game_settings: Res<GameSettings>,
    mut block_set_texts: Query<&mut Text, With<SettingsBlockSetText>>,
) {
    if !game_settings.is_changed() {
        return;
    }

    for mut text in &mut block_set_texts {
        text.0 = game_settings.block_set.label().to_owned();
    }
}

fn handle_settings_back_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<SettingsBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
        }
    }
}

fn handle_quit_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            app_exit.write(AppExit::Success);
        }
    }
}

fn handle_restart_button(
    mut commands: Commands,
    restart_buttons: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    mut game: ResMut<GameModel>,
    game_settings: Res<GameSettings>,
    mut gravity: ResMut<GravityTimer>,
    locked_blocks: Query<Entity, With<LockedBlock>>,
    destroying_blocks: Query<Entity, With<DestroyingBlock>>,
) {
    for interaction in &restart_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if !game.game_over {
            continue;
        }

        for entity in locked_blocks {
            commands.entity(entity).despawn();
        }

        for entity in destroying_blocks {
            commands.entity(entity).despawn();
        }

        *game = GameModel::new(*game_settings);
        gravity.timer.reset();
    }
}

fn handle_main_menu_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<MainMenuButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
        }
    }
}

fn setup_game_main_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(24.0),
                ..default()
            },
            BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.0)),
            MainMenuRoot,
            DespawnOnExit(AppState::MainMenu),
        ))
        .with_children(|main_menu| {
            main_menu
                .spawn((
                    Text::new("BLOCK"),
                    TextFont {
                        font_size: FontSize::Px(72.0),
                        ..default()
                    },
                    TextColor(BevyColor::srgb(0.92, 0.08, 0.04)),
                ))
                .with_children(|title| {
                    title.spawn((
                        TextSpan::new(" OUT"),
                        TextFont {
                            font_size: FontSize::Px(72.0),
                            ..default()
                        },
                        TextColor(BevyColor::srgb(0.08, 0.22, 0.95)),
                    ));
                });

            main_menu
                .spawn((
                    Node {
                        width: px(360.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16.0),
                        padding: UiRect::all(px(20.0)),
                        border: UiRect::all(px(3.0)),
                        ..default()
                    },
                    BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.025)),
                    BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.65)),
                ))
                .with_children(|buttons_container| {
                    buttons_container
                        .spawn((
                            Button,
                            Node {
                                width: percent(100.0),
                                height: px(56.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                            BorderColor::all(BevyColor::srgb(0.2, 1.0, 0.35)),
                            PlayButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("START"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                            ));
                        });

                    buttons_container
                        .spawn((
                            Button,
                            Node {
                                width: percent(100.0),
                                height: px(56.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                            BorderColor::all(BevyColor::srgb(0.2, 0.75, 1.0)),
                            LeaderboardButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("LEADERBOARD"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                            ));
                        });
                    buttons_container
                        .spawn((
                            Button,
                            Node {
                                width: percent(100.0),
                                height: px(56.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                            BorderColor::all(BevyColor::srgb(0.2, 0.75, 1.0)),
                            SettingsButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("SETTINGS"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                            ));
                        });
                    buttons_container
                        .spawn((
                            Button,
                            Node {
                                width: percent(100.0),
                                height: px(56.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                            BorderColor::all(BevyColor::srgb(1.0, 0.15, 0.15)),
                            QuitButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("QUIT"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(1.0, 0.15, 0.15)),
                            ));
                        });
                });
        });
}

fn setup_game_settings(mut commands: Commands, game_settings: Res<GameSettings>) {
    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(20.0),
                ..default()
            },
            BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.0)),
            GameSettingsRoot,
            DespawnOnExit(AppState::Settings),
        ))
        .with_children(|settings| {
            settings.spawn((
                Text::new("GAME SETTINGS"),
                TextFont {
                    font_size: FontSize::Px(52.0),
                    ..default()
                },
                TextColor(BevyColor::srgb(0.2, 0.75, 0.7)),
            ));

            settings
                .spawn((
                    Node {
                        width: px(480.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(12.0),
                        padding: UiRect::all(px(20.0)),
                        border: UiRect::all(px(3.0)),
                        ..default()
                    },
                    BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.025)),
                    BorderColor::all(BevyColor::srgb(0.02, 0.08, 0.65)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("LEVEL"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                    ));

                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|level_selector| {
                            level_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(56.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    LevelAdjustment::Decrease,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("-"),
                                        TextFont {
                                            font_size: FontSize::Px(30.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });

                            level_selector.spawn((
                                Text::new(game_settings.level.to_string()),
                                TextFont {
                                    font_size: FontSize::Px(34.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                                SettingsLevelText,
                            ));

                            level_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(56.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    LevelAdjustment::Increase,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("+"),
                                        TextFont {
                                            font_size: FontSize::Px(30.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });
                        });

                    panel.spawn((
                        Text::new("PIT"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                    ));

                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|pit_selector| {
                            pit_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    PitSizeAdjustment {
                                        direction: SelectionDirection::Previous,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("<"),
                                        TextFont {
                                            font_size: FontSize::Px(28.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });

                            pit_selector.spawn((
                                Text::new(game_settings.pit_size.label()),
                                TextFont {
                                    font_size: FontSize::Px(28.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.92, 0.58, 0.16)),
                                SettingsPitSizeText,
                            ));

                            pit_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    PitSizeAdjustment {
                                        direction: SelectionDirection::Next,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(">"),
                                        TextFont {
                                            font_size: FontSize::Px(28.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });
                        });

                    panel.spawn((
                        Text::new("BLOCK SET"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                    ));

                    panel
                        .spawn(Node {
                            width: percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|block_set_selector| {
                            block_set_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    BlockSetAdjustment {
                                        direction: SelectionDirection::Previous,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("<"),
                                        TextFont {
                                            font_size: FontSize::Px(28.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });

                            block_set_selector.spawn((
                                Text::new(game_settings.block_set.label()),
                                TextFont {
                                    font_size: FontSize::Px(25.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.92, 0.58, 0.16)),
                                SettingsBlockSetText,
                            ));

                            block_set_selector
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(72.0),
                                        height: px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                                    BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                                    BlockSetAdjustment {
                                        direction: SelectionDirection::Next,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(">"),
                                        TextFont {
                                            font_size: FontSize::Px(28.0),
                                            ..default()
                                        },
                                        TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                                    ));
                                });
                        });

                    panel.spawn((
                        Text::new("HIGHER LEVEL = FASTER FALL"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(BevyColor::srgb(0.65, 0.65, 0.65)),
                    ));

                    panel
                        .spawn((
                            Button,
                            Node {
                                width: percent(100.0),
                                height: px(56.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BackgroundColor(BevyColor::srgb(0.04, 0.12, 0.25)),
                            BorderColor::all(BevyColor::srgb(0.1, 0.45, 1.0)),
                            SettingsBackButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("BACK"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                            ));
                        });
                });
        });
}

fn score_for_cleared_planes(cleared_planes_count: usize) -> u64 {
    cleared_planes_count as u64 * SCORE_PER_CLEARED_PLANE
}

#[cfg(test)]
mod model_tests;
