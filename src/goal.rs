use std::collections::HashMap;

/// Status of a goal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
    Cancelled,
}

impl std::fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalStatus::Pending => write!(f, "Pending"),
            GoalStatus::InProgress => write!(f, "InProgress"),
            GoalStatus::Completed => write!(f, "Completed"),
            GoalStatus::Failed => write!(f, "Failed"),
            GoalStatus::Blocked => write!(f, "Blocked"),
            GoalStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A goal with decomposable sub-goals, priority, and progress tracking.
#[derive(Debug, Clone)]
pub struct Goal {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: u32,  // higher = more important
    pub status: GoalStatus,
    pub sub_goals: Vec<Goal>,
    pub progress: f64,  // 0.0 to 1.0
    pub required_compute: f64,
    pub required_memory: f64,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

impl Goal {
    pub fn new(id: &str, name: &str) -> Self {
        let now = Self::now_ms();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            priority: 0,
            status: GoalStatus::Pending,
            sub_goals: vec![],
            progress: 0.0,
            required_compute: 1.0,
            required_memory: 1.0,
            tags: vec![],
            created_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_priority(mut self, p: u32) -> Self {
        self.priority = p;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_resources(mut self, compute: f64, memory: f64) -> Self {
        self.required_compute = compute;
        self.required_memory = memory;
        self
    }

    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn add_sub_goal(&mut self, sub: Goal) {
        self.sub_goals.push(sub);
    }

    /// Start the goal.
    pub fn start(&mut self) {
        if self.status == GoalStatus::Pending {
            self.status = GoalStatus::InProgress;
            self.started_at = Some(Self::now_ms());
        }
    }

    /// Complete the goal.
    pub fn complete(&mut self) {
        self.status = GoalStatus::Completed;
        self.progress = 1.0;
        self.completed_at = Some(Self::now_ms());
    }

    /// Mark the goal as failed.
    pub fn fail(&mut self) {
        self.status = GoalStatus::Failed;
    }

    /// Mark the goal as blocked.
    pub fn block(&mut self) {
        if self.status == GoalStatus::InProgress {
            self.status = GoalStatus::Blocked;
        }
    }

    /// Cancel the goal.
    pub fn cancel(&mut self) {
        self.status = GoalStatus::Cancelled;
    }

    /// Update progress. Clamps to [0.0, 1.0].
    pub fn set_progress(&mut self, p: f64) {
        self.progress = p.clamp(0.0, 1.0);
        if self.status == GoalStatus::Pending {
            self.start();
        }
        if self.progress >= 1.0 && self.status == GoalStatus::InProgress {
            self.complete();
        }
    }

    /// Calculate aggregate progress including sub-goals.
    pub fn aggregate_progress(&self) -> f64 {
        if self.sub_goals.is_empty() {
            return self.progress;
        }
        let total: f64 = self.sub_goals.iter().map(|sg| sg.aggregate_progress()).sum();
        total / self.sub_goals.len() as f64
    }

    /// Check if all sub-goals are completed.
    pub fn all_sub_goals_completed(&self) -> bool {
        self.sub_goals.iter().all(|sg| sg.status == GoalStatus::Completed)
    }

    /// Check if any sub-goal is blocked or failed.
    pub fn has_blocked_sub_goals(&self) -> bool {
        self.sub_goals.iter().any(|sg| {
            sg.status == GoalStatus::Blocked || sg.status == GoalStatus::Failed
        })
    }

    pub fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Decompose a high-level goal into sub-goals based on a strategy.
pub struct GoalDecomposer;

impl GoalDecomposer {
    /// Decompose a goal into N equal sub-goals.
    pub fn decompose_equally(goal: &Goal, count: usize) -> Vec<Goal> {
        let mut sub_goals = Vec::new();
        for i in 0..count {
            let sub = Goal::new(
                &format!("{}_sub_{}", goal.id, i),
                &format!("{} (Part {}/{})", goal.name, i + 1, count),
            )
            .with_priority(goal.priority)
            .with_resources(
                goal.required_compute / count as f64,
                goal.required_memory / count as f64,
            );
            sub_goals.push(sub);
        }
        sub_goals
    }

    /// Create sub-goals from named phases.
    pub fn decompose_phases(goal: &Goal, phases: &[&str]) -> Vec<Goal> {
        let mut sub_goals = Vec::new();
        for (i, phase) in phases.iter().enumerate() {
            let sub = Goal::new(
                &format!("{}_phase_{}", goal.id, i),
                phase,
            )
            .with_priority(goal.priority)
            .with_resources(goal.required_compute, goal.required_memory);
            sub_goals.push(sub);
        }
        sub_goals
    }
}

/// Priority scheduler that orders goals by priority and resource needs.
#[derive(Debug, Clone)]
pub struct PriorityScheduler {
    goals: HashMap<String, Goal>,
}

impl PriorityScheduler {
    pub fn new() -> Self {
        Self { goals: HashMap::new() }
    }

    /// Add a goal to the scheduler.
    pub fn add_goal(&mut self, goal: Goal) -> bool {
        let id = goal.id.clone();
        if self.goals.contains_key(&id) {
            false
        } else {
            self.goals.insert(id, goal);
            true
        }
    }

    /// Remove a goal by ID.
    pub fn remove_goal(&mut self, id: &str) -> Option<Goal> {
        self.goals.remove(id)
    }

    /// Get goals sorted by priority (highest first), then by creation time.
    pub fn scheduled(&self) -> Vec<&Goal> {
        let mut goals: Vec<&Goal> = self.goals.values().collect();
        goals.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        goals
    }

    /// Get the next goal to execute (highest priority non-completed goal).
    pub fn next(&self) -> Option<&Goal> {
        self.scheduled()
            .into_iter()
            .find(|g| g.status == GoalStatus::Pending)
    }

    /// Get only goals that can run with the given available resources.
    pub fn feasible(&self, available_compute: f64, available_memory: f64) -> Vec<&Goal> {
        self.scheduled()
            .into_iter()
            .filter(|g| {
                g.status == GoalStatus::Pending
                && g.required_compute <= available_compute
                && g.required_memory <= available_memory
            })
            .collect()
    }

    /// Get goals by status.
    pub fn by_status(&self, status: &GoalStatus) -> Vec<&Goal> {
        self.goals.values().filter(|g| &g.status == status).collect()
    }

    /// Get a goal by ID.
    pub fn get(&self, id: &str) -> Option<&Goal> {
        self.goals.get(id)
    }

    /// Get a mutable goal by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Goal> {
        self.goals.get_mut(id)
    }

    /// Total number of goals.
    pub fn len(&self) -> usize {
        self.goals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.goals.is_empty()
    }

    /// Summary statistics.
    pub fn summary(&self) -> SchedulerSummary {
        let mut by_status = HashMap::new();
        for g in self.goals.values() {
            *by_status.entry(g.status.clone()).or_insert(0usize) += 1;
        }
        SchedulerSummary { by_status, total: self.goals.len() }
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerSummary {
    pub total: usize,
    pub by_status: HashMap<GoalStatus, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_new() {
        let g = Goal::new("g1", "Test Goal");
        assert_eq!(g.id, "g1");
        assert_eq!(g.status, GoalStatus::Pending);
        assert_eq!(g.progress, 0.0);
    }

    #[test]
    fn goal_lifecycle() {
        let mut g = Goal::new("g1", "Test");
        assert_eq!(g.status, GoalStatus::Pending);
        g.start();
        assert_eq!(g.status, GoalStatus::InProgress);
        assert!(g.started_at.is_some());
        g.complete();
        assert_eq!(g.status, GoalStatus::Completed);
        assert!(g.completed_at.is_some());
        assert_eq!(g.progress, 1.0);
    }

    #[test]
    fn goal_fail() {
        let mut g = Goal::new("g1", "Test");
        g.start();
        g.fail();
        assert_eq!(g.status, GoalStatus::Failed);
    }

    #[test]
    fn goal_block() {
        let mut g = Goal::new("g1", "Test");
        g.start();
        g.block();
        assert_eq!(g.status, GoalStatus::Blocked);
    }

    #[test]
    fn goal_cancel() {
        let mut g = Goal::new("g1", "Test");
        g.cancel();
        assert_eq!(g.status, GoalStatus::Cancelled);
    }

    #[test]
    fn goal_set_progress_auto_start() {
        let mut g = Goal::new("g1", "Test");
        g.set_progress(0.5);
        assert_eq!(g.status, GoalStatus::InProgress);
    }

    #[test]
    fn goal_set_progress_auto_complete() {
        let mut g = Goal::new("g1", "Test");
        g.set_progress(1.0);
        assert_eq!(g.status, GoalStatus::Completed);
    }

    #[test]
    fn goal_progress_clamped() {
        let mut g = Goal::new("g1", "Test");
        g.set_progress(1.5);
        assert_eq!(g.progress, 1.0);
        g.progress = -0.5;
        g.set_progress(g.progress);
        assert_eq!(g.progress, 0.0);
    }

    #[test]
    fn goal_aggregate_progress_sub_goals() {
        let mut g = Goal::new("g1", "Parent");
        let mut sub1 = Goal::new("s1", "Sub1");
        sub1.set_progress(1.0);
        let mut sub2 = Goal::new("s2", "Sub2");
        sub2.set_progress(0.5);
        g.add_sub_goal(sub1);
        g.add_sub_goal(sub2);
        assert!((g.aggregate_progress() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn goal_all_sub_goals_completed() {
        let mut g = Goal::new("g1", "Parent");
        let mut sub1 = Goal::new("s1", "Sub1");
        sub1.complete();
        let mut sub2 = Goal::new("s2", "Sub2");
        sub2.complete();
        g.add_sub_goal(sub1);
        g.add_sub_goal(sub2);
        assert!(g.all_sub_goals_completed());
    }

    #[test]
    fn goal_has_blocked_sub_goals() {
        let mut g = Goal::new("g1", "Parent");
        let mut sub1 = Goal::new("s1", "Sub1");
        sub1.start();
        sub1.block();
        g.add_sub_goal(sub1);
        g.add_sub_goal(Goal::new("s2", "Sub2"));
        assert!(g.has_blocked_sub_goals());
    }

    #[test]
    fn decompose_equally() {
        let parent = Goal::new("g1", "Big Task").with_priority(5).with_resources(10.0, 8.0);
        let subs = GoalDecomposer::decompose_equally(&parent, 3);
        assert_eq!(subs.len(), 3);
        for sub in &subs {
            assert_eq!(sub.priority, 5);
            assert!((sub.required_compute - 10.0/3.0).abs() < 1e-9);
        }
    }

    #[test]
    fn decompose_phases() {
        let parent = Goal::new("g1", "Deploy");
        let subs = GoalDecomposer::decompose_phases(&parent, &["Build", "Test", "Deploy"]);
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0].name, "Build");
        assert_eq!(subs[2].name, "Deploy");
    }

    #[test]
    fn scheduler_add_and_schedule() {
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("low", "Low Priority").with_priority(1));
        sched.add_goal(Goal::new("high", "High Priority").with_priority(10));
        let scheduled = sched.scheduled();
        assert_eq!(scheduled[0].id, "high");
        assert_eq!(scheduled[1].id, "low");
    }

    #[test]
    fn scheduler_next() {
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("low", "Low").with_priority(1));
        sched.add_goal(Goal::new("high", "High").with_priority(10));
        let next = sched.next().unwrap();
        assert_eq!(next.id, "high");
    }

    #[test]
    fn scheduler_feasible() {
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("big", "Big Task").with_resources(100.0, 100.0));
        sched.add_goal(Goal::new("small", "Small Task").with_resources(1.0, 1.0));
        let feasible = sched.feasible(50.0, 50.0);
        assert_eq!(feasible.len(), 1);
        assert_eq!(feasible[0].id, "small");
    }

    #[test]
    fn scheduler_by_status() {
        let mut sched = PriorityScheduler::new();
        let mut g1 = Goal::new("done", "Done");
        g1.complete();
        sched.add_goal(g1);
        sched.add_goal(Goal::new("pending", "Pending"));
        assert_eq!(sched.by_status(&GoalStatus::Completed).len(), 1);
        assert_eq!(sched.by_status(&GoalStatus::Pending).len(), 1);
    }

    #[test]
    fn scheduler_summary() {
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("a", "A"));
        let mut b = Goal::new("b", "B");
        b.complete();
        sched.add_goal(b);
        let summary = sched.summary();
        assert_eq!(summary.total, 2);
        assert_eq!(*summary.by_status.get(&GoalStatus::Pending).unwrap(), 1);
        assert_eq!(*summary.by_status.get(&GoalStatus::Completed).unwrap(), 1);
    }

    #[test]
    fn scheduler_duplicate_add() {
        let mut sched = PriorityScheduler::new();
        assert!(sched.add_goal(Goal::new("g1", "G")));
        assert!(!sched.add_goal(Goal::new("g1", "G2")));
    }

    #[test]
    fn scheduler_remove() {
        let mut sched = PriorityScheduler::new();
        sched.add_goal(Goal::new("g1", "G"));
        let removed = sched.remove_goal("g1");
        assert!(removed.is_some());
        assert!(sched.is_empty());
    }
}
