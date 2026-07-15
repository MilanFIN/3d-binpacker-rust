use crate::common::box_spec::BinBox;

#[derive(Debug, Clone)]
pub struct Solution {
    pub order: Vec<usize>,
    pub score: f64,
    pub solved: Vec<Vec<BinBox>>,
}

impl Solution {
    pub fn new(order: Vec<usize>, score: f64, solved: Vec<Vec<BinBox>>) -> Self {
        Self {
            order,
            score,
            solved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solution_new() {
        let order = vec![1, 2, 3];
        let score = 100.5;
        let solved = vec![vec![]];
        let sol = Solution::new(order.clone(), score, solved);
        
        assert_eq!(sol.order, order);
        assert_eq!(sol.score, score);
        assert_eq!(sol.solved.len(), 1);
    }
}
