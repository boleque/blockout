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

    fn move_by(&mut self, delta: Vec3i) {
        self.position.x += delta.x;
        self.position.y += delta.y;
        self.position.z += delta.z;
    }
}

#[macroquad::main("Blockout")]
async fn main() {
    let well = Well {
        width: 5,
        height: 5,
        depth: 12,
    };

    println!(
        "well size: {} x {} x {}",
        well.width, well.height, well.depth
    );

    let mut piece = Piece {
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

        let mut delta = Vec3i { x: 0, y: 0, z: 0 };

        if is_key_pressed(KeyCode::A) {
            delta.x -= 1;
        }

        if is_key_pressed(KeyCode::D) {
            delta.x += 1;
        }

        if is_key_pressed(KeyCode::S) {
            delta.y -= 1;
        }

        if is_key_pressed(KeyCode::W) {
            delta.y += 1;
        }

        if is_key_pressed(KeyCode::W) {
            delta.z += 1;
        }

        if is_key_pressed(KeyCode::Space) {
            show_line = !show_line;
        }

        if delta.x != 0 || delta.y != 0 || delta.z != 0 {
            piece.move_by(delta);
            println!("piece position: {:?}", piece.position);
        }

        draw_text("Press SPACE to toggle the line", 20.0, 20.0, 20.0, WHITE);

        if show_line {
            draw_line(100.0, 100.0, 300.0, 200.0, 4.0, GREEN);
        }

        next_frame().await;
    }
}
