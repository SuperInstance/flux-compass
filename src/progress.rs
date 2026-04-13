use crate::goal::Goal;
use crate::goal::GoalStatus;
use std::collections::HashMap;

/// Progress snapshot for a goal.
#[derive(Debug, Clone)]
pub struct GoalProgress {
    pub goal_id: String,
    pub goal_name: String,
    pub status: GoalStatus,
    pub direct_progress: f64,
    pub aggregate_progress: f64,
    pub sub_goal_count: usize,
    pub completed_sub_goals: usize,
    pub estimated_completion_ms: Option<u64>,
    pub started_at: Option<u64>,
    pub elapsed_ms: Option<u64>,
}

impl GoalProgress {
    pub fn percent_complete(&self) -> f64 {
        self.aggregate_progress * 100.0
    }
}

/// Tracks progress across multiple goals.
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    goals: HashMap<String, Goal>,
    start_times: HashMap<String, u64>,
    progress_history: HashMap<String, Vec<(u64, f64)>>, // timestamp, progress
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            start_times: HashMap::new(),
            progress_history: HashMap::new(),
        }
    }

    /// Register a goal for tracking.
    pub fn track(&mut self, goal: Goal) -> bool {
        let id = goal.id.clone();
        if self.goals.contains_key(&id) {
            return false;
        }
        self.goals.insert(id.clone(), goal);
        self.progress_history.insert(id, vec![]);
        true
    }

    /// Unregister a goal.
    pub fn untrack(&mut self, id: &str) -> Option<Goal> {
        self.start_times.remove(id);
        self.progress_history.remove(id);
        self.goals.remove(id)
    }

    /// Update progress for a goal.
    pub fn update_progress(&mut self, id: &str, progress: f64) -> bool {
        if let Some(goal) = self.goals.get_mut(id) {
            goal.set_progress(progress);
            let now = Goal::now_ms();
            if let Some(history) = self.progress_history.get_mut(id) {
                history.push((now, progress));
            }
            if goal.started_at.is_some() && self.start_times.get(id).is_none() {
                self.start_times.insert(id.to_string(), now);
            }
            true
        } else {
            false
        }
    }

    /// Get progress snapshot for a specific goal.
    pub fn progress_of(&self, id: &str) -> Option<GoalProgress> {
        let goal = self.goals.get(id)?;
        let now = Self::now_ms();
        let elapsed = goal.started_at.map(|s| now.saturating_sub(s));
        let estimated = self.estimate_completion(id);

        Some(GoalProgress {
            goal_id: goal.id.clone(),
            goal_name: goal.name.clone(),
            status: goal.status.clone(),
            direct_progress: goal.progress,
            aggregate_progress: goal.aggregate_progress(),
            sub_goal_count: goal.sub_goals.len(),
            completed_sub_goals: goal.sub_goals.iter()
                .filter(|sg| sg.status == GoalStatus::Completed)
                .count(),
            estimated_completion_ms: estimated,
            started_at: goal.started_at,
            elapsed_ms: elapsed,
        })
    }

    /// Get progress snapshots for all tracked goals.
    pub fn all_progress(&self) -> Vec<GoalProgress> {
        self.goals.keys()
            .filter_map(|id| self.progress_of(id))
            .collect()
    }

    /// Estimate time to completion based on historical progress rate.
    pub fn estimate_completion(&self, id: &str) -> Option<u64> {
        let goal = self.goals.get(id)?;
        if goal.progress >= 1.0 { return Some(0); }

        let history = self.progress_history.get(id)?;
        if history.len() < 2 { return None; }

        let first = &history[0];
        let last = history.last().unwrap();

        let elapsed = last.0.saturating_sub(first.0);
        if elapsed == 0 { return None; }

        let _progress_delta = last.1 - first.0 as f64 * 0.0; // simplified
        // Calculate rate: progress per millisecond
        let actual_delta = last.1 - first.1;
        if actual_delta <= 0.0 { return None; }

        let remaining = 1.0 - last.1;
        let rate = actual_delta as f64 / elapsed as f64;
        let estimated = (remaining / rate) as u64;
        let now = Self::now_ms();
        Some(now + estimated)
    }

    /// Get goals that are on track (progress increasing over time).
    pub fn on_track(&self) -> Vec<String> {
        self.goals.keys().filter(|id| {
            let history = self.progress_history.get(*id);
            match history {
                Some(h) if h.len() >= 2 => {
                    let last = h.last().unwrap().1;
                    let prev = h[h.len() - 2].1;
                    last >= prev
                }
                _ => false,
            }
        }).cloned().collect()
    }

    /// Get goals that are stalled (progress not increasing).
    pub fn stalled(&self) -> Vec<String> {
        self.goals.keys().filter(|id| {
            let history = self.progress_history.get(*id);
            match history {
                Some(h) if h.len() >= 2 => {
                    let last = h.last().unwrap().1;
                    let prev = h[h.len() - 2].1;
                    last <= prev && last < 1.0
                }
                _ => false,
            }
        }).cloned().collect()
    }

    /// Get overall progress across all goals.
    pub fn overall_progress(&self) -> f64 {
        if self.goals.is_empty() { return 0.0; }
        let total: f64 = self.goals.values()
            .map(|g| g.aggregate_progress())
            .sum();
        total / self.goals.len() as f64
    }

    /// Number of tracked goals.
    pub fn len(&self) -> usize {
        self.goals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.goals.is_empty()
    }

    /// Get a mutable reference to a tracked goal.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Goal> {
        self.goals.get_mut(id)
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_new_empty() {
        let tracker = ProgressTracker::new();
        assert!(tracker.is_empty());
    }

    #[test]
    fn tracker_track_and_untrack() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "Goal 1"));
        assert_eq!(tracker.len(), 1);
        tracker.untrack("g1");
        assert!(tracker.is_empty());
    }

    #[test]
    fn tracker_duplicate_track() {
        let mut tracker = ProgressTracker::new();
        assert!(tracker.track(Goal::new("g1", "G")));
        assert!(!tracker.track(Goal::new("g1", "G2")));
    }

    #[test]
    fn tracker_update_progress() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G"));
        assert!(tracker.update_progress("g1", 0.5));
        let p = tracker.progress_of("g1").unwrap();
        assert!((p.direct_progress - 0.5).abs() < 1e-9);
    }

    #[test]
    fn tracker_update_nonexistent() {
        let mut tracker = ProgressTracker::new();
        assert!(!tracker.update_progress("missing", 0.5));
    }

    #[test]
    fn tracker_progress_auto_complete() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G"));
        tracker.update_progress("g1", 1.0);
        let p = tracker.progress_of("g1").unwrap();
        assert_eq!(p.status, GoalStatus::Completed);
        assert_eq!(p.percent_complete(), 100.0);
    }

    #[test]
    fn tracker_all_progress() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G1"));
        tracker.track(Goal::new("g2", "G2"));
        let all = tracker.all_progress();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn tracker_on_track() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G"));
        tracker.update_progress("g1", 0.3);
        tracker.update_progress("g1", 0.5);
        let on_track = tracker.on_track();
        assert!(on_track.contains(&"g1".to_string()));
    }

    #[test]
    fn tracker_stalled() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G"));
        tracker.update_progress("g1", 0.3);
        tracker.update_progress("g1", 0.3);
        let stalled = tracker.stalled();
        assert!(stalled.contains(&"g1".to_string()));
    }

    #[test]
    fn tracker_overall_progress() {
        let mut tracker = ProgressTracker::new();
        tracker.track(Goal::new("g1", "G1"));
        tracker.track(Goal::new("g2", "G2"));
        tracker.update_progress("g1", 1.0);
        tracker.update_progress("g2", 0.0);
        assert!((tracker.overall_progress() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn tracker_aggregate_progress_with_sub_goals() {
        let mut tracker = ProgressTracker::new();
        let mut g1 = Goal::new("g1", "Parent");
        let mut s1 = Goal::new("s1", "Sub1");
        s1.set_progress(1.0);
        let mut s2 = Goal::new("s2", "Sub2");
        s2.set_progress(0.5);
        g1.add_sub_goal(s1);
        g1.add_sub_goal(s2);
        tracker.track(g1);
        let p = tracker.progress_of("g1").unwrap();
        assert!((p.aggregate_progress - 0.75).abs() < 1e-9);
        assert_eq!(p.completed_sub_goals, 1);
        assert_eq!(p.sub_goal_count, 2);
    }

    #[test]
    fn tracker_empty_overall() {
        let tracker = ProgressTracker::new();
        assert!((tracker.overall_progress() - 0.0).abs() < 1e-9);
    }
}
