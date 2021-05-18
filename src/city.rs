use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync, Model};
use noise::NoiseFn;
use std::convert::TryFrom;

pub fn generate_terrain(
    xsize: GLushort,
    zsize: GLushort,
    height_map: impl Fn(f32, f32) -> f32,
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

const TERRAIN_CHUNKS: u16 = 20;
const TERRAIN_SIZE: u16 = 100;

pub struct City {
    terrain: Vec<Vec<Model>>,
    buildings: Vec<Model>,
    noisefn: noise::Perlin,
}

impl City {
    pub fn new(mut rng: impl rand::Rng, num_buildings: usize) -> Self {
        let noisefn = noise::Perlin::new();
        let mut terrain = Vec::with_capacity(TERRAIN_CHUNKS.into());
        for x in 0..TERRAIN_CHUNKS {
            if terrain.len() <= x.into() {
                terrain.push(Vec::with_capacity(TERRAIN_CHUNKS.into()));
            }

            for z in 0..TERRAIN_CHUNKS {
                terrain[x as usize].push(generate_terrain(
                    TERRAIN_SIZE + 1,
                    TERRAIN_SIZE + 1,
                    |xx, zz| {
                        noisefn.get([
                            (x as f64 + xx as f64 / TERRAIN_SIZE as f64),
                            (z as f64 + zz as f64 / TERRAIN_SIZE as f64),
                        ]) as f32
                            * 100.
                    },
                ));
            }
        }

        let mut buildings = Vec::with_capacity(num_buildings);
        for i in 0..num_buildings {
            let i = i as f32 / (num_buildings - 1) as f32;
            buildings.push(generate_building(glam::vec3(
                4. - rng.gen::<f32>() * i * 2.,
                5. + rng.gen::<f32>() * 3. + i * 10.,
                4. - rng.gen::<f32>() * i * 2.,
            )));
        }

        Self {
            terrain,
            buildings,
            noisefn,
        }
    }

    pub fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        let cam = glam::vec2(sync.get("cam:pos.x"), sync.get("cam:pos.z"));

        // Buildings
        let interval = 15.;
        let radius = 30;
        for x in 0..radius {
            for z in 0..radius {
                // World space position
                let mut pos = (cam / interval + glam::vec2(x as f32, z as f32)
                    - glam::Vec2::splat(radius as f32 / 2.))
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
                    .draw(demo, sync, model);
            }
        }

        // Terrain
        let radius = 8;
        for x in 0..radius {
            for z in 0..radius {
                let ts = TERRAIN_SIZE as f32;
                let viewpos = glam::vec2(x as f32, z as f32);
                let pos = (cam / ts + viewpos - glam::Vec2::splat(radius as f32 / 2.)).floor() * ts;

                let idx = (cam / ts + viewpos) + glam::Vec2::splat(TERRAIN_CHUNKS as f32 / 2.);
                if idx.x >= 0. && (idx.x as usize) < self.terrain.len() {
                    let vec = &self.terrain[idx.x as usize];
                    if idx.y >= 0. && (idx.y as usize) < vec.len() {
                        vec[idx.y as usize].draw(
                            demo,
                            sync,
                            glam::Mat4::from_translation(glam::vec3(pos.x, 0., pos.y)),
                        );
                    }
                }
            }
        }
    }
}
