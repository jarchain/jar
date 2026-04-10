//! Snapshot lookup: find ranking/variances for a given epoch from the cache.
//!
//! Scores (v3 BT output) are the single source of truth for v3. From scores
//! we derive both the ranking (sorted by mu descending) and variances
//! (commitId × sigma2 pairs). For v2, we fall back to ranking.json.

/// A snapshot contains the ranking and optionally variances (for v3).
#[derive(Debug)]
pub struct Snapshot {
    pub ranking: serde_json::Value,
    pub variances: Option<serde_json::Value>,
}

/// Find the last index with epoch < target_epoch and return its commit hash.
fn find_prior_commit_hash(indices: &[serde_json::Value], epoch: u64) -> Option<String> {
    let last = indices
        .iter()
        .rfind(|idx| idx["epoch"].as_u64().map(|e| e < epoch).unwrap_or(false))?;
    last["commitHash"].as_str().map(|s| s.to_string())
}

/// Derive ranking and variances from a scores array (v3 BT output).
/// Scores format: [{"commit": "hash", "mu": N, "sigma2": N}, ...]
/// Returns (ranking sorted by mu desc, variances as [["hash", sigma2], ...]).
fn derive_from_scores(scores: &serde_json::Value) -> Option<Snapshot> {
    let arr = scores.as_array()?;
    // Sort by mu descending to derive ranking
    let mut entries: Vec<(&serde_json::Value, i64)> = arr
        .iter()
        .filter_map(|s| {
            let mu = s["mu"].as_i64()?;
            Some((s, mu))
        })
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let ranking: Vec<serde_json::Value> = entries
        .iter()
        .filter_map(|(s, _)| s.get("commit").cloned())
        .collect();

    // Extract variances as [["hash", sigma2], ...]
    let variances: Vec<serde_json::Value> = arr
        .iter()
        .filter_map(|s| {
            let commit = s.get("commit")?;
            let sigma2 = s.get("sigma2")?;
            Some(serde_json::json!([commit, sigma2]))
        })
        .collect();

    Some(Snapshot {
        ranking: serde_json::json!(ranking),
        variances: Some(serde_json::json!(variances)),
    })
}

/// Find the snapshot for a given epoch, checking scores.json first (v3),
/// then falling back to ranking.json (v2).
///
/// Returns:
/// - `Ok(None)` if no prior index exists (first commit, no ranking needed)
/// - `Ok(Some(snapshot))` if found in either source
/// - `Err(...)` if a prior index exists but its commit hash is missing from
///   both sources (stale cache)
pub fn find(
    indices: &[serde_json::Value],
    ranking_map: &serde_json::Value,
    scores_map: &serde_json::Value,
    epoch: u64,
) -> Result<Option<Snapshot>, Box<dyn std::error::Error>> {
    let commit_hash = match find_prior_commit_hash(indices, epoch) {
        Some(h) => h,
        None => return Ok(None),
    };

    // Try scores.json first (v3)
    if let Some(scores) = scores_map.get(&commit_hash)
        && let Some(snapshot) = derive_from_scores(scores)
    {
        return Ok(Some(snapshot));
    }

    // Fall back to ranking.json (v2). Pass empty variances so v3's
    // select-targets gets a valid array (defaults to BT_SCALE per commit).
    if let Some(ranking) = ranking_map.get(&commit_hash) {
        return Ok(Some(Snapshot {
            ranking: ranking.clone(),
            variances: Some(serde_json::json!([])),
        }));
    }

    Err(format!(
        "cache stale: commit {} not found in ranking.json or scores.json",
        &commit_hash[..8.min(commit_hash.len())]
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_no_prior_index() {
        let indices: Vec<serde_json::Value> = vec![];
        let ranking = serde_json::json!({});
        let scores = serde_json::json!({});
        assert!(find(&indices, &ranking, &scores, 1000).unwrap().is_none());
    }

    #[test]
    fn test_find_all_future() {
        let indices = vec![serde_json::json!({"commitHash": "abc", "epoch": 2000})];
        let ranking = serde_json::json!({"abc": ["abc"]});
        let scores = serde_json::json!({});
        assert!(find(&indices, &ranking, &scores, 1000).unwrap().is_none());
    }

    #[test]
    fn test_find_ranking_fallback() {
        let indices = vec![
            serde_json::json!({"commitHash": "aaa", "epoch": 100}),
            serde_json::json!({"commitHash": "bbb", "epoch": 200}),
        ];
        let ranking = serde_json::json!({
            "aaa": ["aaa"],
            "bbb": ["bbb", "aaa"],
        });
        let scores = serde_json::json!({});
        let snap = find(&indices, &ranking, &scores, 250).unwrap().unwrap();
        assert_eq!(snap.ranking, serde_json::json!(["bbb", "aaa"]));
        assert_eq!(snap.variances, Some(serde_json::json!([])));
    }

    #[test]
    fn test_find_scores_preferred() {
        let indices = vec![serde_json::json!({"commitHash": "aaa", "epoch": 100})];
        let ranking = serde_json::json!({"aaa": ["aaa"]});
        let scores = serde_json::json!({
            "aaa": [
                {"commit": "aaa", "mu": 500, "sigma2": 23000000}
            ]
        });
        let snap = find(&indices, &ranking, &scores, 200).unwrap().unwrap();
        assert_eq!(snap.ranking, serde_json::json!(["aaa"]));
        assert!(snap.variances.is_some());
    }

    #[test]
    fn test_find_scores_sorted_by_mu() {
        let indices = vec![serde_json::json!({"commitHash": "aaa", "epoch": 100})];
        let ranking = serde_json::json!({});
        let scores = serde_json::json!({
            "aaa": [
                {"commit": "bbb", "mu": -100, "sigma2": 20000000},
                {"commit": "aaa", "mu": 500, "sigma2": 23000000}
            ]
        });
        let snap = find(&indices, &ranking, &scores, 200).unwrap().unwrap();
        // aaa has higher mu, should come first
        assert_eq!(snap.ranking, serde_json::json!(["aaa", "bbb"]));
    }

    #[test]
    fn test_find_stale_cache_errors() {
        let indices = vec![serde_json::json!({"commitHash": "abc12345", "epoch": 100})];
        let ranking = serde_json::json!({});
        let scores = serde_json::json!({});
        let err = find(&indices, &ranking, &scores, 200).unwrap_err();
        assert!(err.to_string().contains("cache stale"));
        assert!(err.to_string().contains("abc12345"));
    }
}
