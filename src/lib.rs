
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

#[derive(Clone, Debug, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

pub struct Compass {
    pub heading: f64,
    pub target_heading: f64,
    pub angular_velocity: f64,
    pub turn_rate: f64,
    pub pos: Vec2,
}

fn normalize(deg: f64) -> f64 {
    let d = deg % 360.0;
    if d < 0.0 { d + 360.0 } else { d }
}

impl Compass {
    pub fn new(heading_deg: f64) -> Self {
        Self {
            heading: normalize(heading_deg),
            target_heading: heading_deg,
            angular_velocity: 0.0,
            turn_rate: 90.0,
            pos: Vec2 { x: 0.0, y: 0.0 },
        }
    }

    pub fn set_heading(&mut self, deg: f64) {
        self.heading = normalize(deg);
    }

    pub fn set_target(&mut self, deg: f64) {
        self.target_heading = normalize(deg);
    }

    pub fn tick(&mut self, dt: f64) -> bool {
        let delta = diff(self.heading, self.target_heading);
        if delta.abs() < 0.01 && self.angular_velocity.abs() < 0.01 {
            self.angular_velocity = 0.0;
            self.heading = self.target_heading;
            return true;
        }
        // Spring-damper towards target
        let force = delta * 4.0;
        let damping = -self.angular_velocity * 3.0;
        let accel = force + damping;
        self.angular_velocity += accel * dt;
        // Clamp angular velocity
        let max_vel = self.turn_rate;
        self.angular_velocity = self.angular_velocity.clamp(-max_vel, max_vel);
        self.heading = normalize(self.heading + self.angular_velocity * dt);
        false
    }

    pub fn direction(&self) -> Direction {
        let h = normalize(self.heading + 22.5);
        match (h / 45.0) as usize {
            0 => Direction::N,
            1 => Direction::NE,
            2 => Direction::E,
            3 => Direction::SE,
            4 => Direction::S,
            5 => Direction::SW,
            6 => Direction::W,
            _ => Direction::NW,
        }
    }

    pub fn facing(&self, target: f64, tolerance: f64) -> bool {
        diff(self.heading, target).abs() <= tolerance
    }

    pub fn offset(&self, distance: f64) -> Vec2 {
        let f = forward(self.heading);
        Vec2 {
            x: f.x * distance,
            y: f.y * distance,
        }
    }
}

/// Angular difference from `from` to `to` in [-180, 180].
pub fn diff(from: f64, to: f64) -> f64 {
    let d = normalize(to) - normalize(from);
    if d > 180.0 { d - 360.0 } else if d < -180.0 { d + 360.0 } else { d }
}

/// Unit vector for a heading in degrees (0=N, 90=E).
pub fn forward(heading: f64) -> Vec2 {
    let rad = normalize(heading).to_radians();
    Vec2 { x: rad.sin(), y: -rad.cos() }
}

pub fn distance(a: Vec2, b: Vec2) -> f64 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

pub fn angle_between(from: Vec2, to: Vec2) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    normalize(dx.atan2(-dy).to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert!((normalize(0.0) - 0.0).abs() < 1e-9);
        assert!((normalize(360.0) - 0.0).abs() < 1e-9);
        assert!((normalize(-90.0) - 270.0).abs() < 1e-9);
        assert!((normalize(450.0) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn test_diff() {
        assert!((diff(0.0, 90.0) - 90.0).abs() < 1e-9);
        assert!((diff(0.0, 270.0) - (-90.0)).abs() < 1e-9);
        assert!((diff(90.0, 0.0) - (-90.0)).abs() < 1e-9);
        assert!((diff(350.0, 10.0) - 20.0).abs() < 1e-9);
        assert!((diff(10.0, 350.0) - (-20.0)).abs() < 1e-9);
        assert!((diff(180.0, 180.0)).abs() < 1e-9);
    }

    #[test]
    fn test_diff_wraps_correctly() {
        assert!((diff(359.0, 1.0) - 2.0).abs() < 1e-9);
        assert!((diff(1.0, 359.0) - (-2.0)).abs() < 1e-9);
        assert!((diff(170.0, 190.0) - 20.0).abs() < 1e-9);
        assert!((diff(190.0, 170.0) - (-20.0)).abs() < 1e-9);
    }

    #[test]
    fn test_forward() {
        let n = forward(0.0);
        assert!((n.x.abs() < 1e-9) && ((n.y + 1.0).abs() < 1e-9));
        let e = forward(90.0);
        assert!(((e.x - 1.0).abs() < 1e-9) && (e.y.abs() < 1e-9));
        let s = forward(180.0);
        assert!((s.x.abs() < 1e-9) && ((s.y - 1.0).abs() < 1e-9));
        let w = forward(270.0);
        assert!(((w.x + 1.0).abs() < 1e-9) && (w.y.abs() < 1e-9));
    }

    #[test]
    fn test_forward_is_unit() {
        let f = forward(42.0);
        assert!((f.x.hypot(f.y) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_distance() {
        assert!((distance(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 3.0, y: 4.0 }) - 5.0).abs() < 1e-9);
        assert!((distance(Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: 1.0, y: 1.0 }) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_angle_between() {
        // East of origin
        let a = angle_between(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 1.0, y: 0.0 });
        assert!((a - 90.0).abs() < 1e-9);
        // North of origin
        let b = angle_between(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 0.0, y: -1.0 });
        assert!((b - 0.0).abs() < 1e-9);
        // South of origin
        let c = angle_between(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 0.0, y: 1.0 });
        assert!((c - 180.0).abs() < 1e-9);
    }

    #[test]
    fn test_compass_new() {
        let c = Compass::new(90.0);
        assert!((c.heading - 90.0).abs() < 1e-9);
    }

    #[test]
    fn test_set_heading_normalizes() {
        let mut c = Compass::new(0.0);
        c.set_heading(450.0);
        assert!((c.heading - 90.0).abs() < 1e-9);
    }

    #[test]
    fn test_set_target() {
        let mut c = Compass::new(0.0);
        c.set_target(180.0);
        assert!((c.target_heading - 180.0).abs() < 1e-9);
    }

    #[test]
    fn test_direction_cardinals() {
        assert_eq!(Compass::new(0.0).direction(), Direction::N);
        assert_eq!(Compass::new(90.0).direction(), Direction::E);
        assert_eq!(Compass::new(180.0).direction(), Direction::S);
        assert_eq!(Compass::new(270.0).direction(), Direction::W);
    }

    #[test]
    fn test_direction_ordinates() {
        assert_eq!(Compass::new(45.0).direction(), Direction::NE);
        assert_eq!(Compass::new(135.0).direction(), Direction::SE);
        assert_eq!(Compass::new(225.0).direction(), Direction::SW);
        assert_eq!(Compass::new(315.0).direction(), Direction::NW);
    }

    #[test]
    fn test_facing() {
        let c = Compass::new(10.0);
        assert!(c.facing(5.0, 10.0));
        assert!(c.facing(15.0, 10.0));
        assert!(!c.facing(25.0, 10.0));
    }

    #[test]
    fn test_offset() {
        let c = Compass::new(0.0); // North
        let v = c.offset(10.0);
        assert!((v.x.abs() < 1e-9) && ((v.y + 10.0).abs() < 1e-9));
    }

    #[test]
    fn test_tick_converges() {
        let mut c = Compass::new(0.0);
        c.set_target(90.0);
        for _ in 0..1000 {
            if c.tick(0.016) { break; }
        }
        assert!(c.facing(c.target_heading, 1.0));
    }

    #[test]
    fn test_tick_arrives() {
        let mut c = Compass::new(0.0);
        c.set_target(0.0);
        assert!(c.tick(0.016));
    }

    #[test]
    fn test_tick_large_turn() {
        let mut c = Compass::new(0.0);
        c.set_target(270.0); // Shortest path: -90
        for _ in 0..2000 {
            if c.tick(0.016) { break; }
        }
        assert!((c.heading - 270.0).abs() < 1.0);
    }

    #[test]
    fn test_vec2_copy() {
        let v = Vec2 { x: 1.0, y: 2.0 };
        let v2 = v;
        assert_eq!(v2.x, 1.0);
        assert_eq!(v2.y, 2.0);
    }
}
