use crate::glesv2::{self, types::*, Buffer, RcGl};
use crate::Scene;
use cgmath::{InnerSpace, Vector3};

pub struct Terrain {
    gl: RcGl,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    count: GLint,
}

impl Terrain {
    pub fn new(
        gl: RcGl,
        xsize: GLushort,
        zsize: GLushort,
        height_map: fn(f32, f32) -> f32,
    ) -> Self {
        let vertex_buffer = Buffer::new(gl.clone(), glesv2::ARRAY_BUFFER);
        let mut geometry = Vec::with_capacity(xsize as usize * zsize as usize);

        for x in 0i32..xsize as i32 {
            for z in 0i32..zsize as i32 {
                let x = (x - xsize as i32 / 2) as f32;
                let z = -(z - zsize as i32 / 2) as f32;
                geometry.push(Vector3::new(x, height_map(x, z), z)); // Position

                let pos1 = Vector3::new(x, height_map(x, z - 1.), z - 1.);
                let pos2 = Vector3::new(x - 1., height_map(x - 1., z), z);
                let pos3 = geometry.last().unwrap();

                let u = pos2 - pos1;
                let v = pos3 - pos1;

                geometry.push(u.cross(v).normalize()); // Normal
            }
        }

        vertex_buffer.bind();
        vertex_buffer.data(&geometry, glesv2::STATIC_DRAW);

        let index_buffer = Buffer::new(gl.clone(), glesv2::ELEMENT_ARRAY_BUFFER);
        let mut indices: Vec<GLushort> = Vec::new();

        for i in 0..xsize - 1 {
            for j in 0..zsize {
                indices.push(i * zsize + j); // A
                indices.push(i * zsize + zsize + j); // B
            }

            // Degenerate triangle(s?)
            indices.push(i * zsize + zsize - 1); // C
            indices.push(i * zsize + zsize - 1); // C
            indices.push((i + 1) * zsize); // D
            indices.push((i + 1) * zsize); // D
        }

        index_buffer.bind();
        index_buffer.data(&indices, glesv2::STATIC_DRAW);

        Self {
            gl,
            vertex_buffer,
            index_buffer,
            count: indices.len() as GLint,
        }
    }

    pub fn render(&self, scene: &Scene, lightpos: Vec<f32>) {
        let program = scene
            .resources
            .program("./gouraud.vert ./flatshade.frag")
            .unwrap();

        unsafe {
            self.gl.UseProgram(program.handle());

            self.gl.UniformMatrix4fv(
                program.uniform_location("u_Projection").unwrap(),
                1,
                glesv2::FALSE,
                scene.projection.as_ptr(),
            );
            self.gl.UniformMatrix4fv(
                program.uniform_location("u_View").unwrap(),
                1,
                glesv2::FALSE,
                scene.view.as_ptr(),
            );
            self.gl.Uniform3fv(
                program.uniform_location("u_LightPosition").unwrap(),
                (lightpos.len() / 3) as GLsizei,
                lightpos.as_ptr(),
            );
        }

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_normal = program.attrib_location("a_Normal").unwrap() as GLuint;

        self.vertex_buffer.bind();
        self.index_buffer.bind();

        unsafe {
            self.gl.EnableVertexAttribArray(index_pos);
            self.gl.EnableVertexAttribArray(index_normal);
        }

        let float_size = std::mem::size_of::<GLfloat>();
        let stride = float_size as GLsizei * 6;
        unsafe {
            self.gl.VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                0 as *const GLvoid,
            );
            self.gl.VertexAttribPointer(
                index_normal,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (float_size * 3) as *const GLvoid,
            );

            self.gl.DrawElements(
                glesv2::TRIANGLE_STRIP,
                self.count,
                glesv2::UNSIGNED_SHORT,
                0 as *const GLvoid,
            );
        }
    }
}