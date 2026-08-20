use super::*;

fn test_block(position: Vec3i) -> Block {
    Block {
        position,
        color: Color::Cyan,
        material: Material::Metal,
    }
}

fn well_with_blocks(width: i32, height: i32, depth: i32, positions: &[Vec3i]) -> Well {
    let mut well = Well::new(width, height, depth);

    for position in positions {
        well.place_block(test_block(*position));
    }

    well
}

fn figure_with_blocks(pivot: Vec3i, positions: &[Vec3i]) -> Figure {
    Figure {
        kind: FigureKind::I,
        pivot,
        blocks: positions.iter().copied().map(test_block).collect(),
    }
}

#[test]
fn figure_bag_returns_every_kind_once_before_refill() {
    let mut bag = FigureBag::new(BlockSet::Flat, PitSize::Classic);
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
fn block_sets_contain_the_expected_figure_groups() {
    let flat = BlockSet::Flat.figure_kinds();
    let basic_3d = BlockSet::Basic3d.figure_kinds();
    let extended = BlockSet::Extended.figure_kinds();

    assert_eq!(flat.len(), 7);
    assert_eq!(basic_3d.len(), 3);
    assert_eq!(extended.len(), 10);
    assert!(flat.iter().all(|kind| !basic_3d.contains(kind)));
    assert!(
        extended
            .iter()
            .all(|kind| flat.contains(kind) || basic_3d.contains(kind))
    );
}

#[test]
fn selected_settings_create_matching_game_model() {
    let settings = GameSettings {
        level: 4,
        pit_size: PitSize::Wide,
        block_set: BlockSet::Basic3d,
    };
    let game = GameModel::new(settings);

    assert_eq!(
        (game.well.width, game.well.height, game.well.depth),
        (5, 5, 14)
    );
    assert!(
        BlockSet::Basic3d
            .figure_kinds()
            .contains(&game.active_figure.kind)
    );
    assert!(
        BlockSet::Basic3d
            .figure_kinds()
            .contains(&game.next_figure.kind)
    );
}

#[test]
fn every_figure_spawns_inside_four_by_four_well() {
    let well = Well::new(WELL_WIDTH, WELL_HEIGHT, 12);
    let kinds = [
        FigureKind::I,
        FigureKind::O,
        FigureKind::T,
        FigureKind::L,
        FigureKind::J,
        FigureKind::S,
        FigureKind::Z,
        FigureKind::Tripod,
        FigureKind::ScrewLeft,
        FigureKind::ScrewRight,
    ];

    for kind in kinds {
        let figure = Figure::new(kind, Color::Cyan, Material::Metal);

        assert!(
            well.can_place_figure(&figure),
            "{kind:?} must spawn inside a {WELL_WIDTH}x{WELL_HEIGHT} well"
        );
    }
}

#[test]
fn preview_is_centered_by_figure_bounds() {
    let figure = Figure::new(FigureKind::L, Color::Cyan, Material::Metal);
    let translations = figure
        .blocks
        .iter()
        .map(|block| preview_block_translation(*block, &figure, 1.0))
        .collect::<Vec<_>>();
    let min = translations
        .iter()
        .copied()
        .reduce(Vec3::min)
        .expect("figure has blocks");
    let max = translations
        .iter()
        .copied()
        .reduce(Vec3::max)
        .expect("figure has blocks");

    assert_eq!((min + max) * 0.5, Vec3::ZERO);
}

#[test]
fn game_camera_distance_scales_with_well_entrance() {
    let four_by_four = Well::new(4, 4, 12);
    let six_by_six = Well::new(6, 6, 12);

    assert_eq!(game_camera_z_for_well(&four_by_four), -8.5);
    assert_eq!(game_camera_z_for_well(&six_by_six), -12.5);
    assert!(game_camera_z_for_well(&four_by_four) > game_camera_z_for_well(&six_by_six));
}

#[test]
fn gravity_gets_faster_as_level_increases() {
    assert_eq!(gravity_seconds_for_level(0), gravity_seconds_for_level(1));
    assert_eq!(gravity_seconds_for_level(11), gravity_seconds_for_level(10));

    for level in MIN_LEVEL..MAX_LEVEL {
        assert!(gravity_seconds_for_level(level) > gravity_seconds_for_level(level + 1));
    }
}

#[test]
fn score_is_based_on_cleared_plane_count() {
    assert_eq!(score_for_cleared_planes(0), 0);
    assert_eq!(score_for_cleared_planes(1), 100);
    assert_eq!(score_for_cleared_planes(2), 200);
    assert_eq!(score_for_cleared_planes(4), 400);
}

#[test]
fn clear_full_planes_removes_multiple_planes() {
    let mut well = well_with_blocks(
        2,
        1,
        4,
        &[
            Vec3i { x: 0, y: 0, z: 1 },
            Vec3i { x: 0, y: 0, z: 2 },
            Vec3i { x: 1, y: 0, z: 2 },
            Vec3i { x: 0, y: 0, z: 3 },
            Vec3i { x: 1, y: 0, z: 3 },
        ],
    );

    let cleared_planes = well.clear_full_planes();

    assert_eq!(cleared_planes.len(), 2);
    assert_eq!(well.occupied_count(), 1);
    assert!(well.is_occupied(Vec3i { x: 0, y: 0, z: 3 }));
}

#[test]
fn clearing_full_plane_removes_it_and_shifts_blocks_above() {
    let mut well = well_with_blocks(
        2,
        2,
        4,
        &[
            Vec3i { x: 0, y: 0, z: 2 },
            Vec3i { x: 1, y: 0, z: 2 },
            Vec3i { x: 0, y: 1, z: 2 },
            Vec3i { x: 1, y: 1, z: 2 },
            Vec3i { x: 0, y: 0, z: 1 },
            Vec3i { x: 1, y: 1, z: 3 },
        ],
    );

    assert!(well.clear_plane(2).is_some());
    assert!(!well.is_occupied(Vec3i { x: 0, y: 0, z: 1 }));
    assert!(well.is_occupied(Vec3i { x: 0, y: 0, z: 2 }));
    assert!(well.is_occupied(Vec3i { x: 1, y: 1, z: 3 }));
    assert_eq!(well.occupied_count(), 2);
}

#[test]
fn plane_is_full_when_all_its_slots_are_occupied() {
    let well = well_with_blocks(
        2,
        2,
        3,
        &[
            Vec3i { x: 0, y: 0, z: 2 },
            Vec3i { x: 1, y: 0, z: 2 },
            Vec3i { x: 0, y: 1, z: 2 },
            Vec3i { x: 1, y: 1, z: 2 },
        ],
    );

    assert!(well.is_plane_full(2));
    assert!(!well.is_plane_full(1));
    assert!(!well.is_plane_full(-1));
    assert!(!well.is_plane_full(3));
}

#[test]
fn locking_figure_preserves_block_position_color_and_material() {
    let mut well = Well::new(5, 5, 12);
    let figure = figure_with_blocks(
        Vec3i { x: 2, y: 3, z: 5 },
        &[Vec3i { x: 2, y: 3, z: 5 }, Vec3i { x: 3, y: 3, z: 5 }],
    );

    well.lock_figure(&figure);

    assert_eq!(
        well.block_at(Vec3i { x: 2, y: 3, z: 5 }),
        Some(test_block(Vec3i { x: 2, y: 3, z: 5 }))
    );
    assert!(well.is_occupied(Vec3i { x: 3, y: 3, z: 5 }));
    assert_eq!(well.occupied_count(), 2);
}

#[test]
fn well_rejects_figure_overlapping_occupied_block() {
    let well = well_with_blocks(5, 5, 12, &[Vec3i { x: 3, y: 3, z: 0 }]);
    let figure = figure_with_blocks(
        Vec3i { x: 2, y: 3, z: 0 },
        &[Vec3i { x: 2, y: 3, z: 0 }, Vec3i { x: 3, y: 3, z: 0 }],
    );

    assert!(!well.can_place_figure(&figure));
}

#[test]
fn well_can_place_figure_using_world_positions() {
    let well = Well::new(5, 5, 12);
    let mut figure = figure_with_blocks(
        Vec3i { x: 3, y: 3, z: 0 },
        &[
            Vec3i { x: 3, y: 3, z: 0 },
            Vec3i { x: 4, y: 3, z: 0 },
            Vec3i { x: 4, y: 4, z: 0 },
        ],
    );

    assert!(well.can_place_figure(&figure));

    figure.move_by(Vec3i { x: 1, y: 0, z: 0 });

    assert!(!well.can_place_figure(&figure));
}

#[test]
fn well_contains_only_positions_inside_bounds() {
    let well = Well::new(5, 5, 12);

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
fn entrance_kick_allows_i_figure_to_rotate_into_depth_at_spawn() {
    let well = Well::new(6, 6, 12);
    let figure = Figure::new(FigureKind::I, Color::Cyan, Material::Metal);

    let rotated =
        rotated_figure_with_entrance_kick(&well, &figure, Axis::Y, RotationDirection::Positive)
            .expect("I figure should rotate into the well at the entrance");
    let min_z = rotated.blocks.iter().map(|block| block.position.z).min();
    let max_z = rotated.blocks.iter().map(|block| block.position.z).max();

    assert_eq!(rotated.pivot.z, 2);
    assert_eq!(min_z, Some(0));
    assert_eq!(max_z, Some(3));
    assert!(well.can_place_figure(&rotated));
}

#[test]
fn opposite_rotations_restore_figure() {
    let well = Well::new(6, 6, 12);
    let figure = Figure::new(FigureKind::L, Color::Cyan, Material::Metal);

    let rotated =
        rotated_figure_with_entrance_kick(&well, &figure, Axis::Z, RotationDirection::Positive)
            .and_then(|figure| {
                rotated_figure_with_entrance_kick(
                    &well,
                    &figure,
                    Axis::Z,
                    RotationDirection::Negative,
                )
            })
            .expect("opposite rotations should fit inside an empty well");

    assert_eq!(rotated, figure);
}

#[test]
fn single_key_rotation_cycles_through_unique_orientations() {
    let well = Well::new(WELL_WIDTH, WELL_HEIGHT, 12);
    let mut figure = Figure::new(FigureKind::I, Color::Cyan, Material::Metal);
    figure.move_by(Vec3i { x: 0, y: 0, z: 4 });

    let orientation_count = unique_figure_orientations(figure.kind).len();
    let initial_orientation = orientation_signature(&figure);
    let mut visited_orientations = Vec::new();

    for _ in 0..orientation_count {
        let orientation = normalized_orientation(&orientation_signature(&figure));
        assert!(!visited_orientations.contains(&orientation));
        visited_orientations.push(orientation);
        figure = figure_with_next_orientation(&well, &figure)
            .expect("every I orientation should fit in an empty well");
    }

    assert_eq!(orientation_count, 3);
    assert_eq!(orientation_signature(&figure), initial_orientation);
}

#[test]
fn four_rotations_restore_figure() {
    let original = Figure::new(FigureKind::T, Color::Cyan, Material::Metal);

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let mut rotated = original.clone();

        for _ in 0..4 {
            rotated.rotate_90(axis);
        }

        assert_eq!(
            rotated, original,
            "four rotations around {axis:?} must restore the active figure"
        );
    }
}
