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
use std::collections::{HashMap, HashSet};

const DESTROYING_BLOCK_LIFETIME_SECONDS: f32 = 0.8;
const SCORE_PER_CLEARED_PLANE: u64 = 100;
const PREVIEW_RENDER_LAYER: usize = 1;

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
    score: u64,
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
struct LockedBlock {
    position: Vec3i,
}

#[derive(Component)]
struct GameOverText;

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
struct CubesPlacedText;

#[derive(Component)]
struct RestartButton;

#[derive(Component)]
struct MainMenuButton;

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
    fn material_for(&self, block: Block) -> Handle<StandardMaterial> {
        self.materials
            .get(&(block.color, block.material))
            .expect("every block appearance must have a visual material")
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
    fn new() -> Self {
        let mut figure_bag = FigureBag::new();
        let active_figure = figure_bag.next_figure();
        let next_figure = figure_bag.next_figure();

        Self {
            well: Well::new(6, 6, 12),
            active_figure: active_figure,
            next_figure: next_figure,
            show_line: true,
            game_over: false,
            figure_bag: figure_bag,
            score: 0,
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

fn make_block_material(base_color: BevyColor, material: Material) -> StandardMaterial {
    match material {
        Material::Metal => StandardMaterial {
            base_color,
            metallic: 1.0,
            perceptual_roughness: 0.2,
            ..default()
        },
        Material::Rubber => StandardMaterial {
            base_color,
            metallic: 0.0,
            perceptual_roughness: 0.9,
            ..default()
        },
        Material::Crystal => StandardMaterial {
            base_color,
            metallic: 0.0,
            perceptual_roughness: 0.1,
            specular_transmission: 0.8,
            diffuse_transmission: 0.2,
            thickness: 0.7,
            ior: 1.5,
            ..default()
        },
        Material::Neon => StandardMaterial {
            base_color,
            emissive: base_color.to_linear() * 4.0,
            metallic: 0.0,
            perceptual_roughness: 0.2,
            ..default()
        },
    }
}

// Boundary between the logical integer grid and Bevy's floating-point world.
// One logical cell currently corresponds to one Bevy world unit.
fn logical_position_to_bevy_translation(position: Vec3i) -> Vec3 {
    Vec3::new(position.x as f32, position.y as f32, position.z as f32)
}

fn preview_block_translation(block: Block, figure_pivot: Vec3i, scale: f32) -> Vec3 {
    let local_position = block.position.relative_to(figure_pivot);

    logical_position_to_bevy_translation(local_position) * scale
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
        .insert_resource(GameModel::new())
        .insert_resource(ClearColor(BevyColor::srgb(0.0, 0.0, 0.0)))
        .insert_resource(GravityTimer {
            timer: Timer::from_seconds(0.7, TimerMode::Repeating),
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                apply_gravity,
                sync_figure_position,
                sync_next_figure_preview,
                sync_game_over_text,
                draw_well,
                animate_destroying_blocks,
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
    mut images: ResMut<Assets<Image>>,
) {
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
            Transform::from_xyz(well_center_x, well_center_y, -12.0).looking_at(
                Vec3::new(well_center_x, well_center_y, well_center_z),
                Vec3::Y,
            ),
            GameCamera,
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
            Transform::from_xyz(0.0, 0.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
            RenderLayers::layer(PREVIEW_RENDER_LAYER),
            PreviewCamera,
        ))
        .id();

    commands.spawn((Camera2d, IsDefaultUiCamera, UiCamera));

    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(BevyColor::srgb(0.0, 0.0, 0.0)),
            GameScreenRoot,
        ))
        .with_children(|root| {
            // LeftPanel
            root.spawn((
                Node {
                    width: percent(22.0),
                    height: percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(24.0)),
                    row_gap: px(28.0),
                    border: UiRect::right(px(2.0)),
                    ..default()
                },
                BackgroundColor(BevyColor::srgb(0.01, 0.02, 0.04)),
                BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
            ))
            .with_children(|left_panel| {
                // NEXT BLOCK
                left_panel
                    .spawn((
                        Node {
                            width: percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(12.0),
                            padding: UiRect::bottom(px(18.0)),
                            border: UiRect::bottom(px(1.0)),
                            ..default()
                        },
                        BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
                    ))
                    .with_children(|next_block_section| {
                        next_block_section.spawn((
                            Text::new("NEXT BLOCK"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                        ));

                        next_block_section.spawn((
                            Node {
                                width: percent(100.0),
                                height: px(190.0),
                                border: UiRect::all(px(2.0)),
                                ..default()
                            },
                            BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
                            BackgroundColor(BevyColor::BLACK),
                            ViewportNode::new(preview_camera),
                        ));
                    });

                // PIT
                left_panel
                    .spawn((
                        Node {
                            width: percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(6.0),
                            padding: UiRect::bottom(px(16.0)),
                            border: UiRect::bottom(px(1.0)),
                            ..default()
                        },
                        BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
                    ))
                    .with_children(|pit_section| {
                        pit_section.spawn((
                            Text::new("PIT"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                        ));

                        pit_section.spawn((
                            Text::new(format!(
                                "{} × {} × {}",
                                game.well.width, game.well.height, game.well.depth
                            )),
                            TextFont {
                                font_size: FontSize::Px(26.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                        ));
                    });

                // BLOCK SET
                left_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(6.0),
                        ..default()
                    })
                    .with_children(|block_set_section| {
                        block_set_section.spawn((
                            Text::new("BLOCK SET"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                        ));

                        block_set_section.spawn((
                            Text::new("FLAT"),
                            TextFont {
                                font_size: FontSize::Px(30.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                        ));
                    });
            });

            // GameViewportArea
            root.spawn((
                Node {
                    flex_grow: 1.0,
                    height: percent(100.0),
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(BevyColor::BLACK),
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
                        Visibility::Visible,
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
            // RightPanel
            root.spawn((
                Node {
                    width: percent(22.0),
                    height: percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(24.0)),
                    row_gap: px(28.0),
                    border: UiRect::left(px(2.0)),
                    ..default()
                },
                BackgroundColor(BevyColor::srgb(0.01, 0.02, 0.04)),
                BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
            ))
            .with_children(|right_panel| {
                // LOGO
                right_panel
                    .spawn((
                        Node {
                            width: percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            padding: UiRect::bottom(px(18.0)),
                            border: UiRect::bottom(px(2.0)),
                            ..default()
                        },
                        BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
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
                // SCORE
                right_panel.spawn((
                    Text::new("SCORE"),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                ));
                right_panel.spawn((
                    Text::new(format!("{:06}", game.score)),
                    TextFont {
                        font_size: FontSize::Px(30.0),
                        ..default()
                    },
                    TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                    ScoreText,
                ));
                // CUBES PLACED
                right_panel
                    .spawn((
                        Node {
                            width: percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(6.0),
                            padding: UiRect::bottom(px(16.0)),
                            border: UiRect::bottom(px(1.0)),
                            ..default()
                        },
                        BorderColor::all(BevyColor::srgb(0.1, 0.35, 0.9)),
                    ))
                    .with_children(|cubes_section| {
                        cubes_section.spawn((
                            Text::new("CUBES PLACED"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                        ));

                        cubes_section.spawn((
                            Text::new(format!("{:03}", game.well.occupied_count())),
                            TextFont {
                                font_size: FontSize::Px(30.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                            CubesPlacedText,
                        ));
                    });
                // LEVEL
                right_panel
                    .spawn(Node {
                        width: percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(6.0),
                        ..default()
                    })
                    .with_children(|level_section| {
                        level_section.spawn((
                            Text::new("LEVEL"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 0.75, 1.0)),
                        ));

                        level_section.spawn((
                            Text::new("0"),
                            TextFont {
                                font_size: FontSize::Px(30.0),
                                ..default()
                            },
                            TextColor(BevyColor::srgb(0.2, 1.0, 0.35)),
                        ));
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
    ));

    let block_colors = [
        (Color::Cyan, BevyColor::srgb(0.2, 0.8, 1.0)),
        (Color::Orange, BevyColor::srgb(1.0, 0.4, 0.1)),
        (Color::Green, BevyColor::srgb(0.2, 0.9, 0.3)),
        (Color::Purple, BevyColor::srgb(0.7, 0.2, 1.0)),
        (Color::Yellow, BevyColor::srgb(1.0, 0.85, 0.1)),
    ];
    let block_material_kinds = [
        Material::Metal,
        Material::Rubber,
        Material::Crystal,
        Material::Neon,
    ];
    let mut block_materials = HashMap::new();

    for (color, base_color) in block_colors {
        for material in block_material_kinds {
            let visual_material = materials.add(make_block_material(base_color, material));
            block_materials.insert((color, material), visual_material);
        }
    }

    let block_visuals = BlockVisualAssets {
        mesh: meshes.add(Cuboid::new(0.9, 0.9, 0.9)),
        materials: block_materials,
    };

    let block_mesh = block_visuals.mesh.clone();

    for (index, block) in game.active_figure.blocks.iter().enumerate() {
        let world_position = block.position;
        let block_material = block_visuals.material_for(*block);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(block_material),
            Transform::from_translation(logical_position_to_bevy_translation(world_position)),
            FigureBlockIndex { index },
        ));
    }

    for (index, block) in game.next_figure.blocks.iter().enumerate() {
        let preview_scale = 0.7;
        let preview_translation =
            preview_block_translation(*block, game.next_figure.pivot, preview_scale);
        let preview_material = block_visuals.material_for(*block);

        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(preview_material),
            Transform::from_translation(preview_translation).with_scale(Vec3::splat(preview_scale)),
            RenderLayers::layer(PREVIEW_RENDER_LAYER),
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
    locked_blocks: Query<(Entity, &LockedBlock)>,
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

        let cleared_planes: Vec<Plane> = game.well.clear_full_planes();
        let earned_score = score_for_cleared_planes(cleared_planes.len());
        game.score += earned_score;
        if earned_score > 0 {
            info!(
                "earned score: {}, total score: {}",
                earned_score, game.score
            );
        }

        let cleared_positions = cleared_planes
            .iter()
            .flat_map(|plane| plane.blocks.iter().flatten())
            .map(|block| block.position)
            .collect::<HashSet<_>>();

        for (entity, locked_block) in &locked_blocks {
            if cleared_positions.contains(&locked_block.position) {
                commands.entity(entity).despawn();
            }
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
                ));
            }
        }

        if cleared_planes.len() > 0 {
            info!("cleared {} planes", cleared_planes.len());
        }

        info!("active_figure locked at {:?}", game.active_figure.pivot);
        info!("occupied cell count: {}", game.well.occupied_count());
        info!("cleared planes: {}", cleared_planes.len());

        for block in &locked_figure.blocks {
            if cleared_positions.contains(&block.position) {
                continue;
            }

            commands.spawn((
                Mesh3d(block_visuals.mesh.clone()),
                MeshMaterial3d(block_visuals.material_for(*block)),
                Transform::from_translation(logical_position_to_bevy_translation(block.position)),
                LockedBlock {
                    position: block.position,
                },
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

        transform.translation = logical_position_to_bevy_translation(block.position);

        material.0 = block_visuals.material_for(block);
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

        transform.translation =
            preview_block_translation(block, game.next_figure.pivot, preview_scale);

        material.0 = block_visuals.material_for(block);
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

fn draw_well(mut gizmos: Gizmos, game: Res<GameModel>) {
    let min_x = -0.5;
    let max_x = game.well.width as f32 - 0.5;
    let min_y = -0.5;
    let max_y = game.well.height as f32 - 0.5;
    let entrance_z = -0.5;
    let bottom_z = game.well.depth as f32 - 0.5;
    let wall_color = BevyColor::srgba(0.15, 0.45, 0.65, 0.45);
    let entrance_color = BevyColor::srgb(0.25, 0.8, 1.0);
    let bottom_color = BevyColor::srgba(0.25, 0.65, 0.85, 0.7);

    for z_index in 0..=game.well.depth {
        let z = z_index as f32 - 0.5;
        let color = if z_index == 0 {
            entrance_color
        } else {
            wall_color
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
            wall_color,
        );
        gizmos.line(
            Vec3::new(x, max_y, entrance_z),
            Vec3::new(x, max_y, bottom_z),
            wall_color,
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
            wall_color,
        );
        gizmos.line(
            Vec3::new(max_x, y, entrance_z),
            Vec3::new(max_x, y, bottom_z),
            wall_color,
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
    locked_blocks: Query<(Entity, &LockedBlock)>,
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
        let earned_score = score_for_cleared_planes(cleared_planes.len());
        game.score += earned_score;
        if earned_score > 0 {
            info!(
                "earned score: {}, total score: {}",
                earned_score, game.score
            );
        }

        let cleared_positions = cleared_planes
            .iter()
            .flat_map(|plane| plane.blocks.iter().flatten())
            .map(|block| block.position)
            .collect::<HashSet<_>>();

        for (entity, locked_block) in &locked_blocks {
            if cleared_positions.contains(&locked_block.position) {
                commands.entity(entity).despawn();
            }
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
                ));
            }
        }

        if cleared_planes.len() > 0 {
            info!("cleared {} planes", cleared_planes.len());
        }

        for block in &locked_figure.blocks {
            if cleared_positions.contains(&block.position) {
                continue;
            }

            commands.spawn((
                Mesh3d(block_visuals.mesh.clone()),
                MeshMaterial3d(block_visuals.material_for(*block)),
                Transform::from_translation(logical_position_to_bevy_translation(block.position)),
                LockedBlock {
                    position: block.position,
                },
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

fn score_for_cleared_planes(cleared_planes_count: usize) -> u64 {
    cleared_planes_count as u64 * SCORE_PER_CLEARED_PLANE
}

#[cfg(test)]
mod model_tests;
