use crate::glesv2::{self, types::*};
use crate::Demo;
use glam::{Mat4, Vec3};

struct Building {
    model_matrix: Mat4,
    vertex_buffer: glesv2::Buffer,
    vertex_count: GLint,
}

impl Building {
    fn new() {
        
    }

    fn render(&mut self, demo: &Demo) {
        
    }
}

struct City {
    buildings: Vec<Building>,
}

impl City {
    fn render(&mut self, demo: &Demo) {
        for building in &mut self.buildings {
            building.render(demo);
        }
    }
}
