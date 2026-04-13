use std::collections::HashMap;

/// Condition that can be evaluated against a context.
#[derive(Debug, Clone)]
pub enum Condition {
    /// Always true.
    Always,
    /// Always false.
    Never,
    /// Check if a context key equals a value.
    Eq(String, String),
    /// Check if a context key is greater than a numeric threshold.
    Gt(String, f64),
    /// Check if a context key is less than a numeric threshold.
    Lt(String, f64),
    /// Logical AND of two conditions.
    And(Box<Condition>, Box<Condition>),
    /// Logical OR of two conditions.
    Or(Box<Condition>, Box<Condition>),
    /// Logical NOT.
    Not(Box<Condition>),
}

impl Condition {
    /// Evaluate this condition against a context map.
    pub fn evaluate(&self, context: &HashMap<String, String>) -> bool {
        match self {
            Condition::Always => true,
            Condition::Never => false,
            Condition::Eq(key, value) => {
                context.get(key).map(|v| v == value).unwrap_or(false)
            }
            Condition::Gt(key, threshold) => {
                context.get(key)
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v > *threshold)
                    .unwrap_or(false)
            }
            Condition::Lt(key, threshold) => {
                context.get(key)
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v < *threshold)
                    .unwrap_or(false)
            }
            Condition::And(a, b) => a.evaluate(context) && b.evaluate(context),
            Condition::Or(a, b) => a.evaluate(context) || b.evaluate(context),
            Condition::Not(c) => !c.evaluate(context),
        }
    }
}

/// An action to take when a decision path is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub params: HashMap<String, String>,
}

impl Action {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), params: HashMap::new() }
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }
}

/// A node in the decision tree.
#[derive(Debug, Clone)]
pub enum DecisionNode {
    /// Evaluate condition: if true go to `then_branch`, else go to `else_branch`.
    Branch {
        condition: Condition,
        then_branch: Box<DecisionNode>,
        else_branch: Box<DecisionNode>,
    },
    /// Execute a sequence of sub-trees.
    Sequence(Vec<DecisionNode>),
    /// Take an action (leaf node).
    Action(Action),
    /// Default/fallback action.
    Default(Action),
}

/// A decision tree for routing agent behavior.
#[derive(Debug, Clone)]
pub struct DecisionTree {
    root: DecisionNode,
}

impl DecisionTree {
    pub fn new(root: DecisionNode) -> Self {
        Self { root }
    }

    /// Evaluate the decision tree with the given context.
    /// Returns a list of actions to execute.
    pub fn decide(&self, context: &HashMap<String, String>) -> Vec<Action> {
        let mut actions = Vec::new();
        self.evaluate_node(&self.root, context, &mut actions);
        actions
    }

    fn evaluate_node(&self, node: &DecisionNode, context: &HashMap<String, String>, actions: &mut Vec<Action>) {
        match node {
            DecisionNode::Branch { condition, then_branch, else_branch } => {
                if condition.evaluate(context) {
                    self.evaluate_node(then_branch, context, actions);
                } else {
                    self.evaluate_node(else_branch, context, actions);
                }
            }
            DecisionNode::Sequence(nodes) => {
                for n in nodes {
                    self.evaluate_node(n, context, actions);
                }
            }
            DecisionNode::Action(action) => {
                actions.push(action.clone());
            }
            DecisionNode::Default(action) => {
                actions.push(action.clone());
            }
        }
    }

    /// Count the total number of action leaf nodes.
    pub fn count_actions(&self) -> usize {
        self.count_in_node(&self.root)
    }

    fn count_in_node(&self, node: &DecisionNode) -> usize {
        match node {
            DecisionNode::Branch { then_branch, else_branch, .. } => {
                self.count_in_node(then_branch) + self.count_in_node(else_branch)
            }
            DecisionNode::Sequence(nodes) => {
                nodes.iter().map(|n| self.count_in_node(n)).sum()
            }
            DecisionNode::Action(_) | DecisionNode::Default(_) => 1,
        }
    }

    /// Get the maximum depth of the tree.
    pub fn depth(&self) -> usize {
        self.depth_of(&self.root)
    }

    fn depth_of(&self, node: &DecisionNode) -> usize {
        match node {
            DecisionNode::Branch { then_branch, else_branch, .. } => {
                1 + self.depth_of(then_branch).max(self.depth_of(else_branch))
            }
            DecisionNode::Sequence(nodes) => {
                1 + nodes.iter().map(|n| self.depth_of(n)).max().unwrap_or(0)
            }
            DecisionNode::Action(_) | DecisionNode::Default(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_always() {
        assert!(Condition::Always.evaluate(&HashMap::new()));
    }

    #[test]
    fn condition_never() {
        assert!(!Condition::Never.evaluate(&HashMap::new()));
    }

    #[test]
    fn condition_eq_match() {
        let mut ctx = HashMap::new();
        ctx.insert("mode".to_string(), "debug".to_string());
        assert!(Condition::Eq("mode".into(), "debug".into()).evaluate(&ctx));
    }

    #[test]
    fn condition_eq_no_match() {
        let mut ctx = HashMap::new();
        ctx.insert("mode".to_string(), "release".to_string());
        assert!(!Condition::Eq("mode".into(), "debug".into()).evaluate(&ctx));
    }

    #[test]
    fn condition_gt() {
        let mut ctx = HashMap::new();
        ctx.insert("score".to_string(), "85.0".to_string());
        assert!(Condition::Gt("score".into(), 80.0).evaluate(&ctx));
        assert!(!Condition::Gt("score".into(), 90.0).evaluate(&ctx));
    }

    #[test]
    fn condition_lt() {
        let mut ctx = HashMap::new();
        ctx.insert("temp".to_string(), "45.0".to_string());
        assert!(Condition::Lt("temp".into(), 50.0).evaluate(&ctx));
    }

    #[test]
    fn condition_and() {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), "1".to_string());
        ctx.insert("b".to_string(), "2".to_string());
        let cond = Condition::And(
            Box::new(Condition::Eq("a".into(), "1".into())),
            Box::new(Condition::Eq("b".into(), "2".into())),
        );
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn condition_or() {
        let mut ctx = HashMap::new();
        ctx.insert("x".to_string(), "yes".to_string());
        let cond = Condition::Or(
            Box::new(Condition::Eq("x".into(), "yes".into())),
            Box::new(Condition::Eq("x".into(), "no".into())),
        );
        assert!(cond.evaluate(&ctx));
    }

    #[test]
    fn condition_not() {
        let ctx: HashMap<String, String> = HashMap::new();
        assert!(Condition::Not(Box::new(Condition::Eq("missing".into(), "val".into()))).evaluate(&ctx));
    }

    #[test]
    fn tree_simple_branch() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Eq("mode".into(), "fast".into()),
            then_branch: Box::new(DecisionNode::Action(Action::new("sprint"))),
            else_branch: Box::new(DecisionNode::Action(Action::new("walk"))),
        });

        let mut ctx = HashMap::new();
        ctx.insert("mode".to_string(), "fast".to_string());
        let actions = tree.decide(&ctx);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].name, "sprint");
    }

    #[test]
    fn tree_else_branch() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Eq("mode".into(), "fast".into()),
            then_branch: Box::new(DecisionNode::Action(Action::new("sprint"))),
            else_branch: Box::new(DecisionNode::Action(Action::new("walk"))),
        });

        let ctx = HashMap::new();
        let actions = tree.decide(&ctx);
        assert_eq!(actions[0].name, "walk");
    }

    #[test]
    fn tree_sequence() {
        let tree = DecisionTree::new(DecisionNode::Sequence(vec![
            DecisionNode::Action(Action::new("step1")),
            DecisionNode::Action(Action::new("step2")),
        ]));
        let actions = tree.decide(&HashMap::new());
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].name, "step1");
        assert_eq!(actions[1].name, "step2");
    }

    #[test]
    fn tree_nested_branches() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Gt("cpu".into(), 80.0),
            then_branch: Box::new(DecisionNode::Branch {
                condition: Condition::Eq("task".into(), "gpu".into()),
                then_branch: Box::new(DecisionNode::Action(Action::new("use_gpu"))),
                else_branch: Box::new(DecisionNode::Action(Action::new("use_cpu"))),
            }),
            else_branch: Box::new(DecisionNode::Action(Action::new("wait"))),
        });

        let mut ctx = HashMap::new();
        ctx.insert("cpu".to_string(), "90.0".to_string());
        ctx.insert("task".to_string(), "render".to_string());
        let actions = tree.decide(&ctx);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].name, "use_cpu");
    }

    #[test]
    fn tree_count_actions() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Always,
            then_branch: Box::new(DecisionNode::Sequence(vec![
                DecisionNode::Action(Action::new("a")),
                DecisionNode::Action(Action::new("b")),
            ])),
            else_branch: Box::new(DecisionNode::Action(Action::new("c"))),
        });
        assert_eq!(tree.count_actions(), 3);
    }

    #[test]
    fn tree_depth() {
        let tree = DecisionTree::new(DecisionNode::Branch {
            condition: Condition::Always,
            then_branch: Box::new(DecisionNode::Branch {
                condition: Condition::Always,
                then_branch: Box::new(DecisionNode::Action(Action::new("deep"))),
                else_branch: Box::new(DecisionNode::Action(Action::new("mid"))),
            }),
            else_branch: Box::new(DecisionNode::Action(Action::new("shallow"))),
        });
        assert_eq!(tree.depth(), 3);
    }

    #[test]
    fn action_with_params() {
        let action = Action::new("move")
            .with_param("direction", "north")
            .with_param("speed", "fast");
        assert_eq!(action.name, "move");
        assert_eq!(action.params.get("direction").unwrap(), "north");
        assert_eq!(action.params.get("speed").unwrap(), "fast");
    }
}
