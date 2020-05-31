use crate::glesv2::{self, types::*, Buffer, RcGl, UniformValue};
use crate::Scene;
use isosurface::{
    marching_cubes::MarchingCubes,
    math::Vec3,
    source::{HermiteSource, Source},
};

pub struct DistanceFieldModel {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    count: usize,
}

pub struct SDFSource();

impl Source for SDFSource {
    fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let x = 0.5 - x;
        let y = 0.5 - y;
        let z = 0.5 - z;
        (x * x + y * y + z * z).sqrt() - 0.5
    }
}

impl HermiteSource for SDFSource {
    fn sample_normal(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let dist = self.sample(x, y, z);
        Vec3::new(-x * dist, -y * dist, -z * dist)
    }
}

impl DistanceFieldModel {
    pub fn new(gl: RcGl) -> Self {
        let mut mc = MarchingCubes::new(64);
        let (mut vert, mut ind) = (Vec::new(), Vec::new());
        mc.extract_with_normals(&SDFSource(), &mut vert, &mut ind);

        Self {
            vertex_buffer: {
                let vbuf = Buffer::new(gl.clone(), glesv2::ARRAY_BUFFER);
                vbuf.bind();
                vbuf.data(&vert, glesv2::STATIC_DRAW);
                vbuf
            },
            index_buffer: {
                let ibuf = Buffer::new(gl.clone(), glesv2::ELEMENT_ARRAY_BUFFER);
                ibuf.bind();
                ibuf.data(&ind, glesv2::STATIC_DRAW);
                ibuf
            },
            count: vert.len() / 6,
        }
    }

    pub fn render(&self, scene: &Scene) {
        let program = scene
            .resources
            .program("./gouraud.vert ./flatshade.frag")
            .unwrap();

        program.bind(Some(&[
            (
                program.uniform_location("u_Projection").unwrap(),
                UniformValue::Matrix4fv(1, scene.projection.as_ptr()),
            ),
            (
                program.uniform_location("u_View").unwrap(),
                UniformValue::Matrix4fv(1, scene.view.as_ptr()),
            ),
        ]));

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_normal = program.attrib_location("a_Normal").unwrap() as GLuint;

        self.vertex_buffer.bind();
        self.index_buffer.bind();

        let float_size = std::mem::size_of::<GLfloat>();
        let stride = float_size as GLsizei * 6;

        unsafe {
            scene.gl.EnableVertexAttribArray(index_pos);
            scene.gl.EnableVertexAttribArray(index_normal);

            scene.gl.VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                0 as *const GLvoid,
            );
            scene.gl.VertexAttribPointer(
                index_normal,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (float_size * 3) as *const GLvoid,
            );

            scene.gl.DrawElements(
                glesv2::TRIANGLE_STRIP,
                self.count as GLint,
                glesv2::UNSIGNED_SHORT,
                0 as *const GLvoid,
            );
        }
    }
}
