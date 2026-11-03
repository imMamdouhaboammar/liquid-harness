use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditVerdict {
    Passed,
    Rejected,
    RequiresSanitization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub proposal_id: String,
    pub verdict: AuditVerdict,
    pub violations: Vec<String>,
    pub sanitized_content: String,
    pub latency_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRiskReport {
    pub proposal_id: String,
    pub risk_level: MutationRiskLevel,
    pub score: f64,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub findings: Vec<String>,
    pub safe_to_stage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedConfidenceInterval {
    pub mean_delta: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub ci_width: f64,
    pub standard_error: f64,
    pub sample_size: usize,
    pub is_significant_win: bool,
    pub is_significant_regression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageGateDecision {
    pub stage: String,
    pub passed: bool,
    pub stability_score: f64,
    pub confidence_interval: PairedConfidenceInterval,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    Tool,
    Skill,
    Critic,
    Benchmark,
    Memory,
    Hook,
}

impl std::fmt::Display for SlotKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotKind::Tool => write!(f, "tool"),
            SlotKind::Skill => write!(f, "skill"),
            SlotKind::Critic => write!(f, "critic"),
            SlotKind::Benchmark => write!(f, "benchmark"),
            SlotKind::Memory => write!(f, "memory"),
            SlotKind::Hook => write!(f, "hook"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeType {
    Native,
    Process,
    Script,
    Wasm,
    Prompt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    FsRead,
    FsWrite,
    NetEgress,
    ExecProcess,
    EnvRead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRuntimeConfig {
    #[serde(rename = "type")]
    pub runtime_type: PluginRuntimeType,
    pub entrypoint: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub slot: SlotKind,
    pub description: String,
    pub runtime: PluginRuntimeConfig,
    #[serde(default)]
    pub schema_json: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub estimated_tokens: usize,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    Active,
    Audited,
    Disabled,
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDescriptor {
    pub name: String,
    pub kind: SlotKind,
    pub registered_count: usize,
    pub active_plugin_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(alias = "executionHandler")]
    pub execution_handler: String,
    #[serde(default)]
    pub schema_json: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub estimated_tokens: usize,
    #[serde(default)]
    pub source_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentPhase {
    Boot,
    Assembled,
    Scouting,
    Auditing,
    Active,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub phase: AgentPhase,
    pub generation: usize,
    pub active_tools: Vec<ToolDefinition>,
    pub components: HashMap<String, String>,
    pub champion: Option<ChampionRecord>,
    pub rollback_stack: Vec<AgentStateSnapshot>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub generation: usize,
    pub tools: Vec<ToolDefinition>,
    pub score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionRecord {
    pub generation: usize,
    pub proposal_id: String,
    pub track_id: String,
    pub score: f64,
    pub token_footprint: usize,
    pub promoted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub proposal_id: String,
    pub track_id: String,
    pub generator_type: String,
    pub target_component: String,
    pub rationale: String,
    pub diff: String,
    pub proposed_content: String,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPolicy {
    pub screening_min_score: f64,
    pub validation_min_win: f64,
    pub min_stability_threshold: f64,
    pub confidence_level: f64,
}

impl Default for TrackPolicy {
    fn default() -> Self {
        Self {
            screening_min_score: 1.0,
            validation_min_win: 0.05,
            min_stability_threshold: 0.85,
            confidence_level: 0.95,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDefinition {
    pub id: String,
    pub name: String,
    pub target_component: String,
    pub benchmark_adapter: String,
    pub generator: String,
    pub policy: TrackPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageName {
    Screening,
    Validation,
    Holdout,
}

impl std::fmt::Display for StageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageName::Screening => write!(f, "screening"),
            StageName::Validation => write!(f, "validation"),
            StageName::Holdout => write!(f, "holdout"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: StageName,
    pub passed: bool,
    pub score: f64,
    pub duration_us: u64,
    pub sample_count: usize,
    pub stability_score: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub proposal_id: String,
    pub passed: bool,
    pub final_stage_reached: StageName,
    pub screening: Option<StageResult>,
    pub validation: Option<StageResult>,
    pub holdout: Option<StageResult>,
    pub total_duration_us: u64,
    pub verdict_reason: String,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub proposal_id: String,
    pub baseline_tool: ToolDefinition,
    pub candidate_tool: ToolDefinition,
    pub screening_input: Option<String>,
    pub validation_inputs: Option<Vec<String>>,
    pub holdout_inputs: Option<Vec<String>>,
    pub min_validation_win: Option<f64>,
    pub min_stability_threshold: Option<f64>,
}
