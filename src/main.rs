use macroquad::prelude::*;

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

impl Piece {
    fn world_position(&self, local_block: Vec3i) -> Vec3i {
        Vec3i {
            x: self.position.x + local_block.x,
            y: self.position.y + local_block.y,
            z: self.position.z + local_block.z,
        }
    }
}

#[macroquad::main("Blockout")]
async fn main() {
    let piece = Piece {
        position: Vec3i { x: 2, y: 3, z: 0 },
        blocks: vec![
            Vec3i { x: 0, y: 0, z: 0 },
            Vec3i { x: 1, y: 0, z: 0 },
            Vec3i { x: 1, y: 1, z: 0 },
        ],
    };

    for local_block in &piece.blocks {
        let world_block = piece.world_position(*local_block);
        println!("local {local_block:?} -> world {world_block:?}");
    }

    let mut show_line = true;
    loop {
        clear_background(BLACK);

        if is_key_pressed(KeyCode::Space) {
            show_line = !show_line;
        }

        draw_text("Press SPACE to toggle the line", 20.0, 20.0, 20.0, WHITE);

        if show_line {
            draw_line(100.0, 100.0, 300.0, 200.0, 4.0, GREEN);
        }

        next_frame().await;
    }
}
