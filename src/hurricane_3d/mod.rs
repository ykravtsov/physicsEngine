pub mod renderer;

use crate::quaternion::math::FluxQuaternion;

/// A single particle with quaternion state.
/// w = pressure / ether density,  xyz = velocity
#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub z: f32,               // position
    pub flux: FluxQuaternion, // Ψ: w=pressure, xyz=velocity
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32, // color
    pub size: f32,
}
