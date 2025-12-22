use bevy::prelude::*;
use crate::simulation::galaxy::BlackHole;

pub struct EtherVizPlugin;

impl Plugin for EtherVizPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_ether_flow);
    }
}

fn draw_ether_flow(mut gizmos: Gizmos, black_hole_query: Query<&Transform, With<BlackHole>>) {
    let black_hole_pos = black_hole_query.single().translation;
    const PHI_INV_4: f32 = 0.1464466094067262; // φ^{-4}
    let cos_mu = PHI_INV_4.cos();
    let sin_mu = PHI_INV_4.sin();

    let x_min = -50.0;
    let x_max = 50.0;
    let y_min = -20.0;
    let y_max = 20.0;
    let z_min = -50.0;
    let z_max = 50.0;
    let step = 10.0;
    let arrow_scale = 5.0;
    let shaft_ratio = 0.8;
    let head_width = 0.5;

    let mut x = x_min;
    while x <= x_max {
        let mut y = y_min;
        while y <= y_max {
            let mut z = z_min;
            while z <= z_max {
                let pos = Vec3::new(x, y, z);
                let to_center = black_hole_pos - pos;
                let distance = to_center.length().max(0.1);

                // Suction: vector to black hole with magnitude 1.0 / distance
                let suction = to_center.normalize() * (1.0 / distance);

                // Drag: rotate around up-axis (Y)
                let rotated_x = suction.x * cos_mu + suction.z * sin_mu;
                let rotated_z = -suction.x * sin_mu + suction.z * cos_mu;
                let flow = Vec3::new(rotated_x, suction.y, rotated_z);

                // Color based on magnitude
                let magnitude = flow.length();
                let max_magnitude = 10.0; // Approximate max near center
                let min_magnitude = 0.01; // At edges
                let t = ((magnitude - min_magnitude) / (max_magnitude - min_magnitude)).clamp(0.0, 1.0);
                let shaft_color = Color::srgb(t, 0.0, 1.0 - t); // Red for high, blue for low

                // Draw custom arrow
                let direction = flow.normalize();
                let shaft_length = arrow_scale * shaft_ratio;
                let head_length = arrow_scale * (1.0 - shaft_ratio);
                let shaft_end = pos + direction * shaft_length;
                let tip = shaft_end + direction * head_length;

                // Shaft
                gizmos.line(pos, shaft_end, shaft_color);

                // Head
                let perp = Vec3::cross(direction, Vec3::Y).normalize() * head_width;
                let head_left = shaft_end + perp;
                let head_right = shaft_end - perp;
                gizmos.line(tip, head_left, Color::srgb(1.0, 1.0, 0.0));
                gizmos.line(tip, head_right, Color::srgb(1.0, 1.0, 0.0));
                gizmos.line(shaft_end, tip, Color::srgb(1.0, 1.0, 0.0));

                z += step;
            }
            y += step;
        }
        x += step;
    }
}