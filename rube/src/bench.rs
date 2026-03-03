use crate::camera::Camera;
use glam::Vec3;

pub struct Keyframe {
    pub translation: Vec3,
    pub rotations: (f32, f32),
    pub duration: f32,
}

pub struct Benchmarker {
    frame: FrameStats,
    keyframes: Vec<Keyframe>,
    pub playhead: f32,
    dt: f32,
}

impl Benchmarker {
    pub fn new(dt: f32, keyframes: Vec<Keyframe>) -> Self {
        Self {
            frame: FrameStats::default(),
            keyframes,
            playhead: 0.0,
            dt,
        }
    }
}

#[allow(unused)]
pub fn update(bencher: &mut Benchmarker, camera: &mut Camera, dt: f32) {
    let mut current_frame = 0;
    let mut accum = 0.0;
    for (i, frame) in bencher.keyframes.iter().enumerate() {
        if accum + frame.duration > bencher.playhead {
            current_frame = i;
            break;
        }
        accum += frame.duration;
    }
    if current_frame + 1 >= bencher.keyframes.len() {
        bencher.frame.print();
        std::process::exit(0);
    }
    bencher.frame.update(dt);
    let len = bencher.keyframes.len();
    let f0 = &bencher.keyframes[current_frame.saturating_sub(1)];
    let f1 = &bencher.keyframes[current_frame];
    let f2 = &bencher.keyframes[(current_frame + 1).min(len - 1)];
    let f3 = &bencher.keyframes[(current_frame + 2).min(len - 1)];
    let t = (bencher.playhead - accum) / f1.duration;
    camera.translation = catmull_rom_vec3(
        f0.translation,
        f1.translation,
        f2.translation,
        f3.translation,
        t,
    );
    camera.pitch = catmull_rom_f32(
        f0.rotations.0,
        f1.rotations.0,
        f2.rotations.0,
        f3.rotations.0,
        t,
    );
    camera.yaw = catmull_rom_f32(
        f0.rotations.1,
        f1.rotations.1,
        f2.rotations.1,
        f3.rotations.1,
        t,
    );
    bencher.playhead += bencher.dt;
}

// https://en.wikipedia.org/wiki/Centripetal_Catmull%E2%80%93Rom_spline#Code_example_in_Unity_C#
fn catmull_rom_vec3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

// https://en.wikipedia.org/wiki/Centripetal_Catmull%E2%80%93Rom_spline#Code_example_in_Unity_C#
fn catmull_rom_f32(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

struct FrameStats {
    count: usize,
    total_time: f32,
    min_time: f32,
    max_time: f32,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            count: 0,
            total_time: 0.0,
            min_time: f32::MAX,
            max_time: f32::MIN,
        }
    }
}

impl FrameStats {
    pub fn update(&mut self, delta: f32) {
        self.count += 1;
        self.total_time += delta;
        if delta < self.min_time {
            self.min_time = delta;
        }
        if delta > self.max_time {
            self.max_time = delta;
        }
    }

    pub fn print(&self) {
        let avg_time = self.total_time / self.count as f32;
        let avg_fps = 1.0 / avg_time;
        println!("### Frame\n");
        println!("| metric | value |");
        println!("| :--- | :--- |");
        println!("| total | {} |", self.count);
        println!("| fps avg | {:.2} |", avg_fps);
        println!("| min | {:.4} ms |", self.min_time * 1000.0);
        println!("| max | {:.4} ms |", self.max_time * 1000.0);
        println!("| avg | {:.4} ms |\n", avg_time * 1000.0);
    }
}

#[allow(unused)]
pub fn bench1() -> Benchmarker {
    Benchmarker::new(
        1.0 / 30.0,
        vec![
            Keyframe {
                translation: Vec3::new(1.1192523, 1.0224879, 1.0697857),
                rotations: (0.20999885, 7.3650107),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1654801, 1.0356772, 1.0683582),
                rotations: (-0.010001129, 8.090008),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2135956, 1.0746415, 1.1033561),
                rotations: (-0.1400012, 8.415015),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2537198, 1.0541884, 1.1543454),
                rotations: (-0.31000143, 8.065027),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.275604, 1.0383988, 1.1835108),
                rotations: (0.35499853, 6.8100276),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2696228, 1.0383988, 1.1954263),
                rotations: (0.43499845, 9.095041),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2247651, 1.046656, 1.1899443),
                rotations: (0.40999848, 9.580044),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2029787, 1.0714092, 1.1679018),
                rotations: (-0.015001505, 9.335047),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1687198, 1.0971253, 1.136002),
                rotations: (-0.3400014, 8.310042),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.092891, 1.1488992, 1.1237078),
                rotations: (-0.68500113, 6.940038),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0823298, 1.0954899, 1.219892),
                rotations: (-0.1950013, 5.8100343),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1245404, 1.0579189, 1.2782683),
                rotations: (0.30499867, 4.9750304),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1930584, 1.0328698, 1.3038328),
                rotations: (0.46999845, 4.740028),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.2691292, 1.0328698, 1.282391),
                rotations: (0.4649984, 4.0850263),
                duration: 5.0,
            },
        ],
    )
}

#[allow(unused)]
pub fn bench2() -> Benchmarker {
    Benchmarker::new(
        1.0 / 30.0,
        vec![
            Keyframe {
                translation: Vec3::new(1.0327712, 1.0075046, 1.0571574),
                rotations: (0.37499875, 12.6200285),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0327712, 1.0075046, 1.0571574),
                rotations: (0.37499875, 12.6200285),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0401032, 1.0128554, 1.0744975),
                rotations: (0.02999881, 11.980015),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0746073, 1.0128554, 1.0801904),
                rotations: (0.0049987966, 10.98501),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0791152, 1.0128554, 1.0631496),
                rotations: (0.70999855, 11.560013),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0868993, 1.0378584, 1.0531019),
                rotations: (-0.3150012, 12.650021),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0949527, 1.0434023, 1.0387975),
                rotations: (-0.2650012, 13.350018),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1127602, 1.0434023, 1.0410423),
                rotations: (-0.44500104, 14.595031),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1193871, 1.0386447, 1.0505054),
                rotations: (-0.2450012, 14.915037),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.1225107, 1.0271199, 1.0613791),
                rotations: (-0.22500126, 15.905049),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.087119, 1.0133595, 1.0586292),
                rotations: (0.16999874, 15.700034),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0660175, 1.0236588, 1.0592116),
                rotations: (0.9049985, 15.725035),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0561638, 1.0458118, 1.0587474),
                rotations: (1.0899984, 15.755035),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0561638, 1.0691243, 1.0587474),
                rotations: (-0.18500146, 13.355038),
                duration: 5.0,
            },
            Keyframe {
                translation: Vec3::new(1.0823361, 1.1123073, 1.0576305),
                rotations: (-1.3150008, 12.570025),
                duration: 5.0,
            },
        ],
    )
}
