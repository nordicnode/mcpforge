use crate::model::CatalogEntry;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct CatalogSearch {
    matcher: SkimMatcherV2,
}

impl Default for CatalogSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogSearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn search<'a>(
        &self,
        entries: &'a [CatalogEntry],
        query: &str,
    ) -> Vec<(&'a CatalogEntry, i64)> {
        let query = query.trim();
        if query.is_empty() {
            return entries.iter().map(|e| (e, 0)).collect();
        }

        let mut scored: Vec<(&'a CatalogEntry, i64)> = entries
            .iter()
            .filter_map(|entry| {
                let mut best_score = 0;

                if let Some(s) = self.matcher.fuzzy_match(&entry.id, query) {
                    best_score = best_score.max(s * 3);
                }
                if let Some(s) = self.matcher.fuzzy_match(&entry.name, query) {
                    best_score = best_score.max(s * 3);
                }
                if let Some(s) = self.matcher.fuzzy_match(&entry.category, query) {
                    best_score = best_score.max(s * 2);
                }
                for tag in &entry.tags {
                    if let Some(s) = self.matcher.fuzzy_match(tag, query) {
                        best_score = best_score.max(s * 2);
                    }
                }
                if let Some(s) = self.matcher.fuzzy_match(&entry.description, query) {
                    best_score = best_score.max(s);
                }

                if best_score > 0 {
                    Some((entry, best_score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by_key(|a| std::cmp::Reverse(a.1));
        scored
    }
}
