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
