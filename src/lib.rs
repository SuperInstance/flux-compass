
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

pub mod decision;
pub mod goal;
pub mod adaptation;
pub mod progress;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{Condition, DecisionNode, DecisionTree, Action};
    use crate::goal::{Goal, GoalDecomposer, PriorityScheduler};
    use crate::adaptation::{Resources, AdaptationEngine, Outcome, ResourceAwarePlanner};
    use crate::progress::ProgressTracker;
    use std::collections::HashMap;

    // --- Original Compass tests ---
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

    // --- Integration tests ---

    #[test]
    fn integration_decision_tree_routes_goals() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Eq("priority".into(), "high".into()),
            then_branch: Box::new(DecisionNode::Branch {
                condition: Condition::Gt("resources".into(), 50.0),
                then_branch: Box::new(DecisionNode::Action(Action::new("execute_immediately"))),
                else_branch: Box::new(DecisionNode::Action(Action::new("schedule_high"))),
            }),
            else_branch: Box::new(DecisionNode::Action(Action::new("queue_low"))),
        });

        let mut ctx = HashMap::new();
        ctx.insert("priority".to_string(), "high".to_string());
        ctx.insert("resources".to_string(), "80.0".to_string());
        let actions = tree.decide(&ctx);
        assert_eq!(actions[0].name, "execute_immediately");
    }

    #[test]
    fn integration_decompose_and_schedule() {
        let parent = Goal::new("deploy", "Deploy App").with_priority(10);
        let subs = GoalDecomposer::decompose_phases(&parent, &["Build", "Test", "Ship"]);

        let mut sched = PriorityScheduler::new();
        for sub in subs {
            sched.add_goal(sub);
        }
        assert_eq!(sched.len(), 3);
        let next = sched.next().unwrap();
        assert!(next.id.starts_with("deploy_phase_"));
    }

    #[test]
    fn integration_resource_aware_planning() {
        let planner = ResourceAwarePlanner::new(Resources::new(100.0, 100.0, 10.0));
        let goals = vec![
            Goal::new("heavy", "Heavy").with_resources(60.0, 60.0),
            Goal::new("light", "Light").with_resources(20.0, 20.0),
            Goal::new("medium", "Medium").with_resources(30.0, 30.0),
        ];
        let plan = planner.plan(&goals);
        assert_eq!(plan.len(), 2); // heavy + light OR heavy + medium
        assert_eq!(plan[0].id, "heavy"); // first in list
    }

    #[test]
    fn integration_adaptation_cycle() {
        let mut engine = AdaptationEngine::new();
        let goal = Goal::new("task1", "Recurring Task").with_priority(10);

        // Simulate 3 failures
        for _ in 0..3 {
            engine.record(Outcome {
                goal_id: "task1".to_string(),
                success: false,
                duration_ms: 500,
                resources_used: Resources::zero(),
                notes: "timeout".to_string(),
            });
        }

        let adj = engine.suggest_priority_adjustment(&goal);
        assert!(adj.is_some());
        assert_eq!(adj.unwrap().field, "priority");
        assert_eq!(engine.success_rate("task1"), 0.0);
    }

    #[test]
    fn integration_progress_tracking_with_decomposition() {
        let mut tracker = ProgressTracker::new();
        let parent = Goal::new("project", "Project").with_priority(5);
        let subs = GoalDecomposer::decompose_equally(&parent, 4);

        for sub in subs {
            tracker.track(sub);
        }
        tracker.update_progress("project_sub_0", 0.5);
        tracker.update_progress("project_sub_0", 1.0);
        tracker.update_progress("project_sub_1", 0.25);
        tracker.update_progress("project_sub_1", 0.5);
        tracker.update_progress("project_sub_2", 0.0);
        tracker.update_progress("project_sub_3", 0.25);

        let on_track = tracker.on_track();
        assert!(on_track.contains(&"project_sub_0".to_string()));
        assert!(on_track.contains(&"project_sub_1".to_string()));
    }

    #[test]
    fn integration_full_pipeline() {
        // 1. Create and decompose a goal
        let goal = Goal::new("pipeline", "Data Pipeline").with_priority(8).with_resources(50.0, 30.0);
        let phases = GoalDecomposer::decompose_phases(&goal, &["Extract", "Transform", "Load"]);

        // 2. Schedule with priority
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("other", "Other Task").with_priority(3));
        for phase in phases {
            sched.add_goal(phase);
        }

        // 3. Check feasibility
        let planner = ResourceAwarePlanner::new(Resources::new(80.0, 60.0, 10.0));
        let scheduled = sched.scheduled();
        let scheduled_owned: Vec<Goal> = scheduled.iter().map(|g| (*g).clone()).collect();
        let feasible = planner.plan(&scheduled_owned);
        assert!(feasible.len() >= 2); // At least some goals should fit

        // 4. Track progress
        let mut tracker = ProgressTracker::new();
        for g in scheduled {
            tracker.track(g.clone());
        }

        tracker.update_progress("pipeline_phase_0", 1.0);
        tracker.update_progress("pipeline_phase_1", 0.5);

        // 5. Record outcomes
        let mut engine = AdaptationEngine::new();
        engine.record(Outcome {
            goal_id: "pipeline_phase_0".to_string(),
            success: true,
            duration_ms: 200,
            resources_used: Resources::new(10.0, 5.0, 0.0),
            notes: String::new(),
        });

        assert!(engine.success_rate("pipeline_phase_0") > 0.0);
        assert!(tracker.overall_progress() > 0.0);
    }
}
