use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRecord {
    pub proposal_id: String,
    pub track_id: String,
    pub target_component: String,
    pub timestamp: DateTime<Utc>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureFingerprint {
    pub category: String,
    pub occurrence_count: usize,
    pub last_observed: DateTime<Utc>,
    pub sample_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootMemory {
    pub version: String,
    pub total_campaigns: usize,
    pub champions_promoted: usize,
    pub winning_interventions: Vec<InterventionRecord>,
    pub failure_fingerprints: HashMap<String, FailureFingerprint>,
    pub metadata: HashMap<String, String>,
}

impl Default for RootMemory {
    fn default() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("initialized_at".to_string(), Utc::now().to_rfc3339());
        Self {
            version: "2.0.0".to_string(),
            total_campaigns: 0,
            champions_promoted: 0,
            winning_interventions: Vec::new(),
            failure_fingerprints: HashMap::new(),
            metadata,
        }
    }
}

pub struct RootMemoryManager {
    file_path: PathBuf,
}

impl Default for RootMemoryManager {
    fn default() -> Self {
        Self::new(Path::new(".liquid/root_memory.json"))
    }
}

impl RootMemoryManager {
    pub fn new(path: &Path) -> Self {
        Self {
            file_path: path.to_path_buf(),
        }
    }

    pub fn load(&self) -> RootMemory {
        if self.file_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.file_path) {
                if let Ok(memory) = serde_json::from_str::<RootMemory>(&content) {
                    return memory;
                }
            }
        }
        RootMemory::default()
    }

    pub fn save(&self, memory: &RootMemory) -> Result<(), std::io::Error> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(memory)?;
        fs::write(&self.file_path, json)
    }

    pub fn record_champion(
        &self,
        proposal_id: &str,
        track_id: &str,
        target_component: &str,
        score: f64,
    ) -> Result<(), std::io::Error> {
        let mut memory = self.load();
        memory.champions_promoted += 1;
        memory.total_campaigns += 1;

        memory.winning_interventions.push(InterventionRecord {
            proposal_id: proposal_id.to_string(),
            track_id: track_id.to_string(),
            target_component: target_component.to_string(),
            timestamp: Utc::now(),
            score,
        });

        self.save(&memory)
    }

    pub fn record_failure(&self, category: &str, sample_error: &str) -> Result<(), std::io::Error> {
        let mut memory = self.load();
        let entry = memory
            .failure_fingerprints
            .entry(category.to_string())
            .or_insert_with(|| FailureFingerprint {
                category: category.to_string(),
                occurrence_count: 0,
                last_observed: Utc::now(),
                sample_error: sample_error.to_string(),
            });

        entry.occurrence_count += 1;
        entry.last_observed = Utc::now();
        entry.sample_error = sample_error.to_string();

        self.save(&memory)
    }

    pub fn has_failure_fingerprint(&self, pattern: &str) -> bool {
        let memory = self.load();
        memory.failure_fingerprints.keys().any(|k| k.contains(pattern))
    }
}
