//! Seeded sensor simulation: motion, noise, dropout, and scan generation.

use csm_rs::Pose;
use serde::{Deserialize, Serialize};

use crate::config::{GenerationConfig, MatcherConfig};
use crate::scene::{cast_ray, Scene};

/// A generated scan in sensor coordinates plus its world pose.
#[allow(dead_code)] // pose is used by later playback/reference-policy stages
pub struct ScanFrame {
    pub angles: Vec<f64>,
    pub readings: Vec<f64>,
    pub valid: Vec<bool>,
    pub pose: Pose,
}

/// Deterministic xorshift64* generator so recorded seeds replay exactly.
#[derive(Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Approximately standard-normal via the sum of four uniforms.
    pub fn normal(&mut self) -> f64 {
        (self.unit() + self.unit() + self.unit() + self.unit() - 2.0) * 1.1547
    }
}

/// Sensor world pose for a scenario at `step`.
pub fn pose_at(scenario: &str, step: u64, motion: f64) -> Pose {
    let s = step as f64 * motion;
    match scenario {
        "ambiguous_corridor" => Pose::new(0.15 * s, 0.0, 0.02 * s),
        "partial_overlap" => Pose::new(-3.0 + 0.15 * s, 0.0, 0.01 * s),
        _ => {
            let angle = 0.12 * s;
            Pose::new(
                2.5 * angle.cos(),
                2.5 * angle.sin(),
                angle + std::f64::consts::FRAC_PI_2,
            )
        }
    }
}

/// Generate the scan for a specific simulation frame. The per-frame generator
/// is derived from the seed and frame index so frames are independent and
/// replay exactly regardless of request order.
pub fn scan_for(scene: &Scene, pose: Pose, config: &GenerationConfig, frame: u64) -> ScanFrame {
    let mut rng = Rng::new(
        config.seed
            ^ frame
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(0x1234_5678_9ABC_DEF1),
    );
    scan_at(scene, pose, config, &mut rng)
}

/// A per-frame generator for the initial-guess perturbation.
pub fn guess_rng(seed: u64, frame: u64) -> Rng {
    Rng::new(seed ^ frame.wrapping_mul(0x2545_F491_4F6C_DD1D))
}

/// Generate one scan at the given world pose.
pub fn scan_at(scene: &Scene, pose: Pose, config: &GenerationConfig, rng: &mut Rng) -> ScanFrame {
    let count = config.ray_count.max(2);
    let half_span = config.half_span;
    let mut angles = Vec::with_capacity(count);
    let mut readings = Vec::with_capacity(count);
    let mut valid = Vec::with_capacity(count);
    for i in 0..count {
        let local = -half_span + 2.0 * half_span * i as f64 / (count - 1) as f64;
        angles.push(local);
        let world_angle = local + pose.theta;
        let direction = [world_angle.cos(), world_angle.sin()];
        let hit = cast_ray([pose.x, pose.y], direction, &scene.segments);
        let dropped = config.dropout > 0.0 && rng.unit() < config.dropout;
        match hit {
            Some(distance) if !dropped => {
                let noisy = distance + config.noise * rng.normal();
                if noisy > 0.05 {
                    readings.push(noisy);
                    valid.push(true);
                } else {
                    readings.push(f64::NAN);
                    valid.push(false);
                }
            }
            _ => {
                readings.push(f64::NAN);
                valid.push(false);
            }
        }
    }
    ScanFrame {
        angles,
        readings,
        valid,
        pose,
    }
}

/// The true sensor-to-reference transform for a reference pose.
pub fn relative_pose(reference: Pose, sensor: Pose) -> Pose {
    reference.inverse().compose(sensor)
}

/// Push `sensor` further along its travel direction from `reference`.
///
/// This is the overlap control: it moves the sensor through real geometry, so
/// less of the scene is shared. The achieved overlap is measured, not assumed.
pub fn separated_pose(reference: Pose, sensor: Pose, overlap: f64) -> Pose {
    if overlap == 0.0 {
        return sensor;
    }
    let dx = sensor.x - reference.x;
    let dy = sensor.y - reference.y;
    let norm = dx.hypot(dy);
    let (ux, uy) = if norm > 1e-9 {
        (dx / norm, dy / norm)
    } else {
        (sensor.theta.cos(), sensor.theta.sin())
    };
    Pose::new(
        sensor.x + ux * overlap,
        sensor.y + uy * overlap,
        sensor.theta,
    )
}

/// Sensor world pose for a step, separated from a reference pose by the
/// configured overlap.
pub fn sensor_pose(config: &GenerationConfig, reference: Pose, step: u64) -> Pose {
    let sensor = pose_at(&config.scenario, step, config.motion);
    separated_pose(reference, sensor, config.overlap)
}

/// True relative pose for one previous-frame transition.
pub fn step_truth(config: &GenerationConfig, step: u64) -> Pose {
    let previous = pose_at(&config.scenario, step.saturating_sub(1), config.motion);
    let current = sensor_pose(config, previous, step);
    relative_pose(previous, current)
}

/// Accumulated true motion over `steps` previous-frame transitions.
///
/// Truth never depends on matching, so this is cheap to recompute.
pub fn accumulated_truth(config: &GenerationConfig, steps: u64) -> Pose {
    let mut total = Pose::IDENTITY;
    for step in 1..=steps {
        total = total.compose(step_truth(config, step));
    }
    total
}

/// A perturbed initial guess for the matcher.
pub fn initial_guess(truth: Pose, error: f64, rng: &mut Rng) -> Pose {
    if error <= 0.0 {
        return truth;
    }
    Pose::new(
        truth.x + error * rng.normal(),
        truth.y + error * rng.normal(),
        truth.theta + 0.5 * error * rng.normal(),
    )
}

/// Matching result stored in an exported session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionResult {
    pub estimated_pose: [f64; 3],
    pub valid: bool,
    pub termination: String,
    pub iterations: i32,
    pub nvalid: i32,
    pub error: f64,
}

/// A versioned export of a frame request for replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // used by the import/export/replay stage
pub struct SessionRecord {
    pub version: u32,
    pub generation: GenerationConfig,
    pub reference_mode: String,
    pub matcher: MatcherConfig,
    pub reference_angles: Vec<f64>,
    pub reference_readings: Vec<Option<f64>>,
    pub reference_valid: Vec<bool>,
    pub sensor_angles: Vec<f64>,
    pub sensor_readings: Vec<Option<f64>>,
    pub sensor_valid: Vec<bool>,
    pub initial_guess: [f64; 3],
    pub result: SessionResult,
}
