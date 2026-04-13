use crate::goal::Goal;
use crate::decision::DecisionTree;

/// Available system resources.
#[derive(Debug, Clone)]
pub struct Resources {
    pub compute: f64,
    pub memory: f64,
    pub network: f64,
}

impl Resources {
    pub fn new(compute: f64, memory: f64, network: f64) -> Self {
        Self { compute, memory, network }
    }

    pub fn unlimited() -> Self {
        Self { compute: f64::MAX, memory: f64::MAX, network: f64::MAX }
    }

    pub fn zero() -> Self {
        Self { compute: 0.0, memory: 0.0, network: 0.0 }
    }

    /// Check if resources can accommodate a goal's requirements.
    pub fn can_run(&self, goal: &Goal) -> bool {
        self.compute >= goal.required_compute
            && self.memory >= goal.required_memory
    }

    /// Subtract a goal's resource requirements.
    pub fn allocate(&mut self, goal: &Goal) {
        self.compute -= goal.required_compute;
        self.memory -= goal.required_memory;
    }

    /// Add back a goal's resource requirements.
    pub fn release(&mut self, goal: &Goal) {
        self.compute += goal.required_compute;
        self.memory += goal.required_memory;
    }

    /// Check if a goal is feasible and return remaining resources after allocation.
    pub fn remaining_after(&self, goal: &Goal) -> Option<Resources> {
        if self.can_run(goal) {
            Some(Resources {
                compute: self.compute - goal.required_compute,
                memory: self.memory - goal.required_memory,
                network: self.network,
            })
        } else {
            None
        }
    }

    /// Calculate utilization percentage (0-1) for compute.
    pub fn compute_utilization(&self, max: &Resources) -> f64 {
        if max.compute <= 0.0 { return 0.0; }
        (1.0 - self.compute / max.compute).clamp(0.0, 1.0)
    }
}

/// An adaptation record representing an outcome and any adjustment.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub goal_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub resources_used: Resources,
    pub notes: String,
}

/// A strategy adjustment made by the adaptation engine.
#[derive(Debug, Clone)]
pub struct Adjustment {
    pub goal_id: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub reason: String,
}

/// The adaptation engine monitors outcomes and adjusts plans.
#[derive(Debug, Clone)]
pub struct AdaptationEngine {
    outcomes: Vec<Outcome>,
    adjustments: Vec<Adjustment>,
    learning_rate: f64,
}

impl AdaptationEngine {
    pub fn new() -> Self {
        Self {
            outcomes: vec![],
            adjustments: vec![],
            learning_rate: 0.1,
        }
    }

    pub fn with_learning_rate(mut self, rate: f64) -> Self {
        self.learning_rate = rate.clamp(0.01, 1.0);
        self
    }

    /// Record the outcome of executing a goal.
    pub fn record(&mut self, outcome: Outcome) {
        self.outcomes.push(outcome);
    }

    /// Get all recorded outcomes for a specific goal.
    pub fn outcomes_for(&self, goal_id: &str) -> Vec<&Outcome> {
        self.outcomes.iter().filter(|o| o.goal_id == goal_id).collect()
    }

    /// Calculate success rate for a goal (0.0 to 1.0).
    pub fn success_rate(&self, goal_id: &str) -> f64 {
        let outcomes = self.outcomes_for(goal_id);
        if outcomes.is_empty() { return 1.0; }
        let successes = outcomes.iter().filter(|o| o.success).count();
        successes as f64 / outcomes.len() as f64
    }

    /// Calculate average duration for a goal.
    pub fn avg_duration(&self, goal_id: &str) -> Option<f64> {
        let outcomes = self.outcomes_for(goal_id);
        if outcomes.is_empty() { return None; }
        let total: u64 = outcomes.iter().map(|o| o.duration_ms).sum();
        Some(total as f64 / outcomes.len() as f64)
    }

    /// Suggest priority adjustment based on success rate.
    /// Returns an Adjustment if one is recommended.
    pub fn suggest_priority_adjustment(&mut self, goal: &Goal) -> Option<Adjustment> {
        let rate = self.success_rate(&goal.id);
        if rate < 0.5 && self.outcomes_for(&goal.id).len() >= 3 {
            // Lower priority if consistently failing
            let new_priority = (goal.priority as f64 * (1.0 - self.learning_rate)).max(0.0) as u32;
            if new_priority != goal.priority {
                let adj = Adjustment {
                    goal_id: goal.id.clone(),
                    field: "priority".to_string(),
                    old_value: goal.priority.to_string(),
                    new_value: new_priority.to_string(),
                    reason: format!("Low success rate ({:.0}%), reducing priority", rate * 100.0),
                };
                self.adjustments.push(adj.clone());
                return Some(adj);
            }
        }
        None
    }

    /// Suggest resource adjustment based on duration trends.
    pub fn suggest_resource_adjustment(&mut self, goal: &Goal) -> Option<Adjustment> {
        let outcomes = self.outcomes_for(&goal.id);
        if outcomes.len() < 3 { return None; }

        let avg_dur = self.avg_duration(&goal.id).unwrap_or(0.0);
        let slow_outcomes: Vec<_> = outcomes.iter().filter(|o| o.duration_ms as f64 > avg_dur * 1.5).collect();
        if slow_outcomes.len() as f64 / outcomes.len() as f64 > 0.5 {
            let factor = 1.0 + self.learning_rate;
            let new_compute = (goal.required_compute * factor).min(1000.0);
            let adj = Adjustment {
                goal_id: goal.id.clone(),
                field: "compute".to_string(),
                old_value: goal.required_compute.to_string(),
                new_value: new_compute.to_string(),
                reason: format!("Tasks running slow (avg {:.0}ms), increasing compute allocation", avg_dur),
            };
            self.adjustments.push(adj.clone());
            return Some(adj);
        }
        None
    }

    /// Apply all suggested adjustments to a goal and scheduler.
    pub fn apply_adjustments(
        &self,
        goal: &mut Goal,
    ) -> Vec<&Adjustment> {
        let goal_adjustments: Vec<&Adjustment> = self.adjustments.iter()
            .filter(|a| a.goal_id == goal.id)
            .collect();

        for adj in &goal_adjustments {
            match adj.field.as_str() {
                "priority" => {
                    if let Ok(val) = adj.new_value.parse::<u32>() {
                        goal.priority = val;
                    }
                }
                "compute" => {
                    if let Ok(val) = adj.new_value.parse::<f64>() {
                        goal.required_compute = val;
                    }
                }
                "memory" => {
                    if let Ok(val) = adj.new_value.parse::<f64>() {
                        goal.required_memory = val;
                    }
                }
                _ => {}
            }
        }
        goal_adjustments
    }

    /// Total number of recorded outcomes.
    pub fn outcome_count(&self) -> usize {
        self.outcomes.len()
    }

    /// Total number of adjustments.
    pub fn adjustment_count(&self) -> usize {
        self.adjustments.len()
    }

    /// Get all adjustments.
    pub fn adjustments(&self) -> &[Adjustment] {
        &self.adjustments
    }
}

/// Resource-aware planner that combines scheduling with resource constraints.
#[derive(Debug, Clone)]
pub struct ResourceAwarePlanner {
    pub available: Resources,
    pub max_resources: Resources,
}

impl ResourceAwarePlanner {
    pub fn new(available: Resources) -> Self {
        let max = available.clone();
        Self { available, max_resources: max }
    }

    /// Given a list of goals, return those that can be executed with available resources.
    pub fn plan<'a>(&self, goals: &'a [Goal]) -> Vec<&'a Goal> {
        let mut result = Vec::new();
        let mut remaining = self.available.clone();
        for goal in goals {
            if remaining.can_run(goal) {
                result.push(goal);
                remaining.allocate(goal);
            }
        }
        result
    }

    /// Calculate a feasibility score (0.0-1.0) for a set of goals.
    pub fn feasibility_score(&self, goals: &[Goal]) -> f64 {
        if goals.is_empty() { return 1.0; }
        let feasible = self.plan(goals);
        feasible.len() as f64 / goals.len() as f64
    }

    /// Estimate how many goals can run concurrently.
    pub fn max_concurrent(&self, goals: &[Goal]) -> usize {
        self.plan(goals).len()
    }

    /// Get current resource utilization.
    pub fn utilization(&self) -> f64 {
        self.available.compute_utilization(&self.max_resources)
    }

    /// Build a decision tree that routes based on resource availability.
    pub fn resource_decision_tree(&self, goal: &Goal) -> DecisionTree {
        use crate::decision::{Condition, DecisionNode, Action};

        let can_compute = Condition::Gt("available_compute".into(), goal.required_compute);
        let can_memory = Condition::Gt("available_memory".into(), goal.required_memory);
        let both = Condition::And(
            Box::new(can_compute),
            Box::new(can_memory),
        );

        DecisionTree::new(DecisionNode::Branch {
            condition: both,
            then_branch: Box::new(DecisionNode::Action(Action::new("execute_goal")
                .with_param("goal_id", &goal.id))),
            else_branch: Box::new(DecisionNode::Branch {
                condition: Condition::Gt("available_compute".into(), goal.required_compute * 0.5),
                then_branch: Box::new(DecisionNode::Action(Action::new("queue_goal")
                    .with_param("goal_id", &goal.id))),
                else_branch: Box::new(DecisionNode::Action(Action::new("defer_goal")
                    .with_param("goal_id", &goal.id))),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_goal(id: &str, compute: f64, memory: f64) -> Goal {
        Goal::new(id, id).with_resources(compute, memory).with_priority(5)
    }

    // --- Resources tests ---
    #[test]
    fn resources_new() {
        let r = Resources::new(100.0, 50.0, 10.0);
        assert_eq!(r.compute, 100.0);
    }

    #[test]
    fn resources_can_run() {
        let r = Resources::new(100.0, 50.0, 10.0);
        let g = make_goal("g1", 50.0, 30.0);
        assert!(r.can_run(&g));
    }

    #[test]
    fn resources_cannot_run() {
        let r = Resources::new(10.0, 10.0, 10.0);
        let g = make_goal("g1", 50.0, 30.0);
        assert!(!r.can_run(&g));
    }

    #[test]
    fn resources_allocate_release() {
        let mut r = Resources::new(100.0, 50.0, 10.0);
        let g = make_goal("g1", 30.0, 20.0);
        r.allocate(&g);
        assert!((r.compute - 70.0).abs() < 1e-9);
        r.release(&g);
        assert!((r.compute - 100.0).abs() < 1e-9);
    }

    #[test]
    fn resources_remaining_after() {
        let r = Resources::new(100.0, 50.0, 10.0);
        let g = make_goal("g1", 30.0, 20.0);
        let remaining = r.remaining_after(&g).unwrap();
        assert!((remaining.compute - 70.0).abs() < 1e-9);
        assert!((remaining.memory - 30.0).abs() < 1e-9);
    }

    #[test]
    fn resources_remaining_none() {
        let r = Resources::new(10.0, 10.0, 10.0);
        let g = make_goal("g1", 50.0, 20.0);
        assert!(r.remaining_after(&g).is_none());
    }

    // --- AdaptationEngine tests ---
    #[test]
    fn adaptation_record() {
        let mut engine = AdaptationEngine::new();
        engine.record(Outcome {
            goal_id: "g1".to_string(),
            success: true,
            duration_ms: 100,
            resources_used: Resources::new(10.0, 5.0, 0.0),
            notes: String::new(),
        });
        assert_eq!(engine.outcome_count(), 1);
    }

    #[test]
    fn adaptation_success_rate() {
        let mut engine = AdaptationEngine::new();
        for _ in 0..3 {
            engine.record(Outcome {
                goal_id: "g1".to_string(),
                success: true,
                duration_ms: 100,
                resources_used: Resources::zero(),
                notes: String::new(),
            });
        }
        engine.record(Outcome {
            goal_id: "g1".to_string(),
            success: false,
            duration_ms: 200,
            resources_used: Resources::zero(),
            notes: String::new(),
        });
        assert!((engine.success_rate("g1") - 0.75).abs() < 1e-9);
    }

    #[test]
    fn adaptation_suggest_priority_decrease() {
        let mut engine = AdaptationEngine::new();
        for _ in 0..3 {
            engine.record(Outcome {
                goal_id: "g1".to_string(),
                success: false,
                duration_ms: 100,
                resources_used: Resources::zero(),
                notes: String::new(),
            });
        }
        let goal = Goal::new("g1", "Test").with_priority(10);
        let adj = engine.suggest_priority_adjustment(&goal);
        assert!(adj.is_some());
        assert_eq!(adj.unwrap().field, "priority");
    }

    #[test]
    fn adaptation_suggest_resource_increase() {
        let mut engine = AdaptationEngine::new();
        // Fast task
        for _ in 0..3 {
            engine.record(Outcome {
                goal_id: "g1".to_string(),
                success: true,
                duration_ms: 100,
                resources_used: Resources::zero(),
                notes: String::new(),
            });
        }
        // Slow tasks
        for _ in 0..4 {
            engine.record(Outcome {
                goal_id: "g1".to_string(),
                success: true,
                duration_ms: 500,
                resources_used: Resources::zero(),
                notes: String::new(),
            });
        }
        let goal = Goal::new("g1", "Test").with_resources(5.0, 5.0);
        let adj = engine.suggest_resource_adjustment(&goal);
        assert!(adj.is_some());
        assert_eq!(adj.unwrap().field, "compute");
    }

    #[test]
    fn adaptation_apply_adjustments() {
        let mut engine = AdaptationEngine::new();
        for _ in 0..3 {
            engine.record(Outcome {
                goal_id: "g1".to_string(),
                success: false,
                duration_ms: 100,
                resources_used: Resources::zero(),
                notes: String::new(),
            });
        }
        let goal = Goal::new("g1", "Test").with_priority(10);
        engine.suggest_priority_adjustment(&goal);
        let mut goal = Goal::new("g1", "Test").with_priority(10);
        engine.apply_adjustments(&mut goal);
        assert!(goal.priority < 10);
    }

    // --- ResourceAwarePlanner tests ---
    #[test]
    fn planner_plan() {
        let planner = ResourceAwarePlanner::new(Resources::new(100.0, 100.0, 10.0));
        let goals = vec![
            make_goal("big", 80.0, 80.0),
            make_goal("small", 20.0, 20.0),
            make_goal("tiny", 5.0, 5.0),
        ];
        let plan = planner.plan(&goals);
        assert_eq!(plan.len(), 2); // big (80+80 > 100)... wait
        // big: compute=80, memory=80 < 100 ✓
        // remaining: 20, 20
        // small: compute=20, memory=20 ✓
        // remaining: 0, 0
        // tiny: compute=5, memory=5 > 0 ✗
        assert_eq!(plan[0].id, "big");
        assert_eq!(plan[1].id, "small");
    }

    #[test]
    fn planner_feasibility_score() {
        let planner = ResourceAwarePlanner::new(Resources::new(50.0, 50.0, 10.0));
        let goals = vec![
            make_goal("a", 10.0, 10.0),
            make_goal("b", 20.0, 20.0),
            make_goal("c", 100.0, 100.0),
        ];
        let score = planner.feasibility_score(&goals);
        assert!((score - 2.0/3.0).abs() < 1e-9);
    }

    #[test]
    fn planner_max_concurrent() {
        let planner = ResourceAwarePlanner::new(Resources::new(10.0, 10.0, 10.0));
        let goals = vec![
            make_goal("a", 3.0, 3.0),
            make_goal("b", 3.0, 3.0),
            make_goal("c", 3.0, 3.0),
            make_goal("d", 5.0, 5.0),
        ];
        assert_eq!(planner.max_concurrent(&goals), 3);
    }

    #[test]
    fn planner_utilization() {
        let planner = ResourceAwarePlanner::new(Resources::new(50.0, 100.0, 10.0));
        assert!((planner.utilization() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn planner_resource_decision_tree() {
        let planner = ResourceAwarePlanner::new(Resources::new(100.0, 100.0, 10.0));
        let goal = make_goal("g1", 30.0, 30.0);
        let tree = planner.resource_decision_tree(&goal);
        let mut ctx = HashMap::new();
        ctx.insert("available_compute".to_string(), "50.0".to_string());
        ctx.insert("available_memory".to_string(), "50.0".to_string());
        let actions = tree.decide(&ctx);
        assert_eq!(actions[0].name, "execute_goal");
    }

    #[test]
    fn planner_resource_decision_tree_defer() {
        let planner = ResourceAwarePlanner::new(Resources::new(100.0, 100.0, 10.0));
        let goal = make_goal("g1", 80.0, 80.0);
        let tree = planner.resource_decision_tree(&goal);
        let mut ctx = HashMap::new();
        ctx.insert("available_compute".to_string(), "10.0".to_string());
        ctx.insert("available_memory".to_string(), "10.0".to_string());
        let actions = tree.decide(&ctx);
        assert_eq!(actions[0].name, "defer_goal");
    }
}
