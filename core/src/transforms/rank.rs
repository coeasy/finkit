use super::Transform;

/// Rank transform: replaces each value with its rank (1-based) in ascending order.
///
/// Ties are handled by averaging ranks.
/// Output has the same length as input.
pub struct Rank;

impl Transform for Rank {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() {
            return vec![];
        }
        let n = input.len();
        let mut indexed: Vec<(usize, f64)> = input.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut ranks = vec![0.0; n];
        let mut i = 0;
        while i < n {
            let mut j = i;
            while j < n && (indexed[j].1 - indexed[i].1).abs() < 1e-15 {
                j += 1;
            }
            let avg_rank = (i + 1 + j) as f64 / 2.0;
            for item in indexed.iter().take(j).skip(i) {
                ranks[item.0] = avg_rank;
            }
            i = j;
        }
        ranks
    }
}

/// Percentile rank transform: rank / n, scales output to [0, 1].
pub struct PercentileRank;

impl Transform for PercentileRank {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() {
            return vec![];
        }
        let n = input.len() as f64;
        Rank.transform(input).iter().map(|r| r / n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_basic() {
        let data = vec![30.0, 10.0, 20.0, 40.0, 50.0];
        let result = Rank.transform(&data);
        assert_eq!(result, vec![3.0, 1.0, 2.0, 4.0, 5.0]);
    }

    #[test]
    fn test_rank_ties() {
        let data = vec![10.0, 20.0, 20.0, 30.0];
        let result = Rank.transform(&data);
        assert_eq!(result, vec![1.0, 2.5, 2.5, 4.0]);
    }

    #[test]
    fn test_rank_empty() {
        assert!(Rank.transform(&[]).is_empty());
    }

    #[test]
    fn test_rank_single() {
        let result = Rank.transform(&[42.0]);
        assert_eq!(result, vec![1.0]);
    }

    #[test]
    fn test_rank_all_same() {
        let data = vec![5.0, 5.0, 5.0];
        let result = Rank.transform(&data);
        assert_eq!(result, vec![2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_percentile_rank_basic() {
        let data = vec![30.0, 10.0, 20.0, 40.0, 50.0];
        let result = PercentileRank.transform(&data);
        assert!((result[0] - 0.6).abs() < 1e-10);
        assert!((result[1] - 0.2).abs() < 1e-10);
        assert!((result[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_rank_empty() {
        assert!(PercentileRank.transform(&[]).is_empty());
    }
}
