use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync, Model};
use std::convert::TryFrom;

pub fn generate_terrain(
    xsize: GLushort,
    zsize: GLushort,
    height_map: fn(f32, f32) -> f32,
) -> Model {
    let vertex_buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
    let mut geometry = Vec::with_capacity(xsize as usize * zsize as usize);

    for x in 0i32..xsize as i32 {
        for z in 0i32..zsize as i32 {
            let x = (x - xsize as i32 / 2) as f32;
            let z = -(z - zsize as i32 / 2) as f32;
            geometry.push(glam::vec3(x, height_map(x, z), z)); // Position

            let pos1 = glam::vec3(x, height_map(x, z - 1.), z - 1.);
            let pos2 = glam::vec3(x - 1., height_map(x - 1., z), z);
            let pos3 = geometry.last().unwrap();

            let u = pos2 - pos1;
            let v = *pos3 - pos1;

            geometry.push(u.cross(v).normalize()); // Normal
        }
    }

    vertex_buffer.bind();
    vertex_buffer.data(&geometry, glesv2::STATIC_DRAW);

    let index_buffer = glesv2::Buffer::new(glesv2::ELEMENT_ARRAY_BUFFER);
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

    Model {
        mode: glesv2::TRIANGLE_STRIP,
        vertex_buffer,
        index_buffer: Some(index_buffer),
        num_elements: indices.len() as GLint,
    }
}

fn generate_building(size: glam::Vec3) -> Model {
    // Hard coded cube coordinates
    let coords = &[
        -1f32, -1., 1., 1., -1., 1., 1., 1., 1., -1., -1., 1., 1., 1., 1., -1., 1., 1., 1., -1.,
        1., 1., -1., -1., 1., 1., -1., 1., -1., 1., 1., 1., -1., 1., 1., 1., 1., -1., -1., -1.,
        -1., -1., -1., 1., -1., 1., -1., -1., -1., 1., -1., 1., 1., -1., -1., -1., -1., -1., -1.,
        1., -1., 1., 1., -1., -1., -1., -1., 1., 1., -1., 1., -1., -1., -1., -1., 1., -1., -1., 1.,
        -1., 1., -1., -1., -1., 1., -1., 1., -1., -1., 1., -1., 1., 1., 1., 1., 1., 1., 1., -1.,
        -1., 1., 1., 1., 1., -1., -1., 1., -1.,
    ];

    // Hard coded cube normals
    let normals = &[
        0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.,
        0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1.,
        0., 0., -1., 0., 0., -1., 0., 0., -1., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0.,
        -1., 0., 0., -1., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0.,
        0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0.,
    ];

    let transformation =
            // Move the foundations underground
            glam::Mat4::from_translation(glam::vec3(0., -5., 0.)) *
            // Scale in width, height and depth
            glam::Mat4::from_scale(size) *
            // Raise the cube to sit on origin
            glam::Mat4::from_translation(glam::vec3(0., 1., 0.));

    // Transform into an array of structures
    let transformation_normal = glam::Mat3::from(transformation).inverse().transpose();
    let mesh: Vec<glam::Vec3> = coords
        .chunks(3)
        .zip(normals.chunks(3))
        .flat_map(|(coord, normal)| {
            let coord: glam::Vec4 = (coord[0], coord[1], coord[2], 1.).into();
            let normal: glam::Vec3 = (normal[0], normal[1], normal[2]).into();
            let coord = transformation * coord;
            let normal = transformation_normal * normal;
            vec![coord.into(), normal]
        })
        .collect();

    let vertex_buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
    vertex_buffer.bind();
    #[rustfmt::skip]
        vertex_buffer.data(
            &mesh,
            glesv2::STATIC_DRAW,
        );

    Model {
        mode: glesv2::TRIANGLES,
        vertex_buffer,
        index_buffer: None,
        num_elements: GLint::try_from(mesh.len() / 2).unwrap(),
    }
}

pub struct City {
    terrain: Model,
    buildings: Vec<Model>,
    noisefn: noise::Perlin,
}

impl City {
    pub fn new(mut rng: impl rand::Rng, num_buildings: usize) -> Self {
        let mut buildings = Vec::with_capacity(num_buildings);

        for i in 0..num_buildings {
            let i = i as f32 / (num_buildings - 1) as f32;
            buildings.push(generate_building(glam::vec3(
                1.,
                5. + rng.gen::<f32>() * 3. + i * 10.,
                1.,
            )));
        }

        Self {
            terrain: generate_terrain(200, 200, |x, z| 0.),
            buildings,
            noisefn: noise::Perlin::new(),
        }
    }

    pub fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        use noise::NoiseFn;
        let cam = glam::vec2(sync.get("cam:pos.x"), sync.get("cam:pos.z"));

        // Buildings
        let interval = 7.;
        let radius = 50;
        for x in 0..radius {
            for z in 0..radius {
                // World space position
                let mut pos = (cam / interval + glam::vec2(x as f32, z as f32)
                    - glam::Vec2::splat(radius as f32) / 2.)
                    .floor()
                    * interval;

                // noise values for this position
                let np = pos * 10. / (radius as f32 * interval);
                let noise = glam::vec2(
                    self.noisefn.get([np.x as f64, np.y as f64]) as f32,
                    self.noisefn.get([np.x as f64 - 10., np.y as f64 - 10.]) as f32,
                );

                // Add variation to positions
                pos += noise * 2.;

                let model = glam::Mat4::from_translation(glam::vec3(pos.x, 0., pos.y));
                self.buildings[(noise.x * (self.buildings.len() - 1) as f32) as usize]
                    .draw(demo, model);
            }
        }

        self.terrain.draw(
            demo,
            glam::Mat4::from_translation(glam::vec3(cam.x, 0., cam.y)),
        );
    }
}
