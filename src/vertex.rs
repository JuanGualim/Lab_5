//! Vértices en coordenadas de pantalla (x hacia la derecha, y hacia abajo).

use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Color,
}

impl Vertex {
    pub const fn new(x: f32, y: f32, color: Color) -> Self {
        Vertex {
            position: Vec3::new(x, y, 0.0),
            color,
        }
    }
}
