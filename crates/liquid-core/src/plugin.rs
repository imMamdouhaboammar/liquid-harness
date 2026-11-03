use crate::critic::SecurityCritic;
use crate::types::{
    AuditVerdict, PluginManifest, PluginPermission, PluginRuntimeType, PluginStatus, SlotDescriptor,
    SlotKind, ToolDefinition,
};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct PluginExecutionContext {
    pub agent_id: String,
    pub generation: usize,
    pub working_dir: String,
}

impl Default for PluginExecutionContext {
    fn default() -> Self {
        Self {
            agent_id: "agent-default".to_string(),
            generation: 1,
            working_dir: ".".to_string(),
        }
    }
}

pub trait LiquidPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn manifest(&self) -> &PluginManifest;
    fn preflight_check(&self) -> Result<(), String>;
    fn execute(&self, payload: &str, ctx: &PluginExecutionContext) -> Result<String, String>;
}

pub struct ManifestPlugin {
    manifest: PluginManifest,
}

impl ManifestPlugin {
    pub fn new(manifest: PluginManifest) -> Self {
        Self { manifest }
    }
}

impl LiquidPlugin for ManifestPlugin {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn preflight_check(&self) -> Result<(), String> {
        if self.manifest.id.trim().is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }
        if self.manifest.runtime.entrypoint.trim().is_empty() {
            return Err("Plugin entrypoint cannot be empty".to_string());
        }
        Ok(())
    }

    fn execute(&self, payload: &str, _ctx: &PluginExecutionContext) -> Result<String, String> {
        match self.manifest.runtime.runtime_type {
            PluginRuntimeType::Process | PluginRuntimeType::Script => {
                let entrypoint = &self.manifest.runtime.entrypoint;
                let is_windows = cfg!(target_os = "windows");
                let mut cmd = if is_windows {
                    let mut c = Command::new("cmd");
                    c.args(["/C", entrypoint]);
                    c
                } else {
                    let mut c = Command::new("sh");
                    c.args(["-c", entrypoint]);
                    c
                };

                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn process: {}", e))?;

                if !payload.is_empty() {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(payload.as_bytes());
                    }
                }

                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Process execution error: {}", e))?;

                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    Err(format!("Process exited with status {}: {}", output.status, err_msg))
                }
            }
            PluginRuntimeType::Prompt => {
                let template = &self.manifest.runtime.entrypoint;
                if template.contains("{{input}}") {
                    Ok(template.replace("{{input}}", payload))
                } else if !payload.is_empty() {
                    Ok(format!("{}\n\nPayload: {}", template, payload))
                } else {
                    Ok(template.clone())
                }
            }
            PluginRuntimeType::Native | PluginRuntimeType::Wasm => {
                Ok(format!(
                    "[Native/WASM Execution Simulated] id={} payload_len={}",
                    self.manifest.id,
                    payload.len()
                ))
            }
        }
    }
}

pub struct Slot {
    pub name: String,
    pub kind: SlotKind,
    plugins: HashMap<String, (PluginStatus, Arc<dyn LiquidPlugin>)>,
}

impl Slot {
    pub fn new(name: &str, kind: SlotKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            plugins: HashMap::new(),
        }
    }

    pub fn insert(&mut self, status: PluginStatus, plugin: Arc<dyn LiquidPlugin>) {
        self.plugins.insert(plugin.id().to_string(), (status, plugin));
    }

    pub fn get(&self, id: &str) -> Option<&(PluginStatus, Arc<dyn LiquidPlugin>)> {
        self.plugins.get(id)
    }

    pub fn list(&self) -> Vec<(PluginManifest, PluginStatus)> {
        self.plugins
            .values()
            .map(|(status, plugin)| (plugin.manifest().clone(), status.clone()))
            .collect()
    }
}

pub struct SlotRegistry {
    slots: RwLock<HashMap<String, Slot>>,
}

impl Default for SlotRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SlotRegistry {
    pub fn new() -> Self {
        let registry = Self {
            slots: RwLock::new(HashMap::new()),
        };
        registry.seed_default_slots();
        registry
    }

    fn seed_default_slots(&self) {
        let mut slots = self.slots.write().unwrap();
        let defaults = [
            ("capabilities", SlotKind::Tool),
            ("workflows", SlotKind::Skill),
            ("guardrails", SlotKind::Critic),
            ("benchmarks", SlotKind::Benchmark),
            ("storage", SlotKind::Memory),
            ("lifecycle", SlotKind::Hook),
        ];

        for (name, kind) in defaults {
            slots.insert(name.to_string(), Slot::new(name, kind));
        }
    }

    pub fn create_slot(&self, name: &str, kind: SlotKind) -> Result<(), String> {
        let mut slots = self.slots.write().unwrap();
        if slots.contains_key(name) {
            return Err(format!("Slot '{}' already exists", name));
        }
        slots.insert(name.to_string(), Slot::new(name, kind));
        Ok(())
    }

    pub fn register_plugin(
        &self,
        plugin: Arc<dyn LiquidPlugin>,
        critic: &SecurityCritic,
    ) -> Result<PluginStatus, String> {
        plugin.preflight_check()?;
        let manifest = plugin.manifest();

        let audit = critic.audit(&manifest.id, &manifest.description, &manifest.schema_json);
        if audit.verdict == AuditVerdict::Rejected {
            let reason = format!("Security critic rejected plugin metadata: {:?}", audit.violations);
            return Ok(PluginStatus::Rejected(reason));
        }

        if manifest.permissions.contains(&PluginPermission::ExecProcess)
            || manifest.runtime.runtime_type == PluginRuntimeType::Process
        {
            let diff_audit = critic.audit_mutation_diff(
                &manifest.id,
                &format!("+ {}\n", manifest.runtime.entrypoint),
            );
            if !diff_audit.safe_to_stage {
                let reason = format!(
                    "Security critic rejected entrypoint command: {:?}",
                    diff_audit.findings
                );
                return Ok(PluginStatus::Rejected(reason));
            }
        }

        let mut slots = self.slots.write().unwrap();
        let target_slot_name = match manifest.slot {
            SlotKind::Tool => "capabilities",
            SlotKind::Skill => "workflows",
            SlotKind::Critic => "guardrails",
            SlotKind::Benchmark => "benchmarks",
            SlotKind::Memory => "storage",
            SlotKind::Hook => "lifecycle",
        };

        let slot = slots.entry(target_slot_name.to_string()).or_insert_with(|| {
            Slot::new(target_slot_name, manifest.slot)
        });

        let status = PluginStatus::Active;
        slot.insert(status.clone(), plugin);
        Ok(status)
    }

    pub fn load_manifest_from_str(
        &self,
        content: &str,
        critic: &SecurityCritic,
    ) -> Result<(PluginManifest, PluginStatus), String> {
        let manifest: PluginManifest = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse plugin manifest JSON: {}", e))?;

        let plugin = Arc::new(ManifestPlugin::new(manifest.clone()));
        let status = self.register_plugin(plugin, critic)?;
        Ok((manifest, status))
    }

    pub fn load_manifest_file<P: AsRef<Path>>(
        &self,
        path: P,
        critic: &SecurityCritic,
    ) -> Result<(PluginManifest, PluginStatus), String> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .map_err(|e| format!("Failed to read manifest file {}: {}", path_ref.display(), e))?;
        self.load_manifest_from_str(&content, critic)
    }

    pub fn execute(
        &self,
        plugin_id: &str,
        payload: &str,
        ctx: &PluginExecutionContext,
    ) -> Result<String, String> {
        let slots = self.slots.read().unwrap();
        for slot in slots.values() {
            if let Some((status, plugin)) = slot.get(plugin_id) {
                if *status != PluginStatus::Active {
                    return Err(format!("Plugin '{}' is not active: {:?}", plugin_id, status));
                }
                return plugin.execute(payload, ctx);
            }
        }
        Err(format!("Plugin '{}' not found in any registered slot", plugin_id))
    }

    pub fn list_slots(&self) -> Vec<SlotDescriptor> {
        let slots = self.slots.read().unwrap();
        slots
            .values()
            .map(|s| {
                let active_ids = s
                    .plugins
                    .iter()
                    .filter(|(_, (status, _))| *status == PluginStatus::Active)
                    .map(|(id, _)| id.clone())
                    .collect();

                SlotDescriptor {
                    name: s.name.clone(),
                    kind: s.kind,
                    registered_count: s.plugins.len(),
                    active_plugin_ids: active_ids,
                }
            })
            .collect()
    }

    pub fn list_plugins(&self, filter: Option<SlotKind>) -> Vec<(PluginManifest, PluginStatus)> {
        let slots = self.slots.read().unwrap();
        let mut results = Vec::new();
        for slot in slots.values() {
            if let Some(kind) = filter {
                if slot.kind != kind {
                    continue;
                }
            }
            results.extend(slot.list());
        }
        results
    }

    pub fn export_tool_definitions(&self) -> Vec<ToolDefinition> {
        let slots = self.slots.read().unwrap();
        let mut tools = Vec::new();

        if let Some(tool_slot) = slots.get("capabilities") {
            for (status, plugin) in tool_slot.plugins.values() {
                if *status == PluginStatus::Active {
                    let m = plugin.manifest();
                    tools.push(ToolDefinition {
                        id: m.id.clone(),
                        name: m.name.clone(),
                        version: m.version.clone(),
                        description: m.description.clone(),
                        execution_handler: format!("plugin://{}", m.id),
                        schema_json: m.schema_json.clone(),
                        tags: m.tags.clone(),
                        estimated_tokens: m.estimated_tokens,
                        source_uri: format!("slot://capabilities/{}", m.id),
                    });
                }
            }
        }
        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginRuntimeConfig;

    #[test]
    fn test_slot_creation_and_listing() {
        let registry = SlotRegistry::new();
        let slots = registry.list_slots();
        assert_eq!(slots.len(), 6);
        assert!(slots.iter().any(|s| s.name == "capabilities"));

        assert!(registry.create_slot("custom_ai_eval", SlotKind::Benchmark).is_ok());
        let updated = registry.list_slots();
        assert_eq!(updated.len(), 7);
    }

    #[test]
    fn test_manifest_plugin_registration_and_execution() {
        let registry = SlotRegistry::new();
        let critic = SecurityCritic::new();

        let manifest = PluginManifest {
            id: "echo_tool".to_string(),
            name: "Echo Test Tool".to_string(),
            version: "1.0.0".to_string(),
            slot: SlotKind::Tool,
            description: "Safe echo tool for integration verification".to_string(),
            runtime: PluginRuntimeConfig {
                runtime_type: PluginRuntimeType::Process,
                entrypoint: "echo hello-liquid".to_string(),
                timeout_ms: 1000,
            },
            schema_json: r#"{"type":"object"}"#.to_string(),
            tags: vec!["test".to_string()],
            estimated_tokens: 15,
            author: Some("Developer".to_string()),
            permissions: vec![PluginPermission::ExecProcess],
        };

        let plugin = Arc::new(ManifestPlugin::new(manifest));
        let status = registry.register_plugin(plugin, &critic).unwrap();
        assert_eq!(status, PluginStatus::Active);

        let ctx = PluginExecutionContext::default();
        let output = registry.execute("echo_tool", "", &ctx).unwrap();
        assert_eq!(output, "hello-liquid");

        let tools = registry.export_tool_definitions();
        assert!(tools.iter().any(|t| t.id == "echo_tool"));
    }

    #[test]
    fn test_malicious_plugin_rejected_by_critic() {
        let registry = SlotRegistry::new();
        let critic = SecurityCritic::new();

        let manifest = PluginManifest {
            id: "malicious_tool".to_string(),
            name: "Dangerous Tool".to_string(),
            version: "1.0.0".to_string(),
            slot: SlotKind::Tool,
            description: "Dangerous tool attempting deletion".to_string(),
            runtime: PluginRuntimeConfig {
                runtime_type: PluginRuntimeType::Process,
                entrypoint: "rm -rf /tmp/data".to_string(),
                timeout_ms: 1000,
            },
            schema_json: "{}".to_string(),
            tags: vec![],
            estimated_tokens: 10,
            author: None,
            permissions: vec![PluginPermission::ExecProcess],
        };

        let plugin = Arc::new(ManifestPlugin::new(manifest));
        let status = registry.register_plugin(plugin, &critic).unwrap();
        assert!(matches!(status, PluginStatus::Rejected(_)));
    }

    #[test]
    fn test_prompt_skill_slot_execution() {
        let registry = SlotRegistry::new();
        let critic = SecurityCritic::new();

        let manifest = PluginManifest {
            id: "clarify_skill".to_string(),
            name: "Requirement Clarifier".to_string(),
            version: "1.0.0".to_string(),
            slot: SlotKind::Skill,
            description: "Synthesizes clear requirements from input".to_string(),
            runtime: PluginRuntimeConfig {
                runtime_type: PluginRuntimeType::Prompt,
                entrypoint: "Instructions: Clarify the following intent: {{input}}".to_string(),
                timeout_ms: 500,
            },
            schema_json: "{}".to_string(),
            tags: vec!["skill".to_string(), "cognitive".to_string()],
            estimated_tokens: 25,
            author: None,
            permissions: vec![],
        };

        let plugin = Arc::new(ManifestPlugin::new(manifest));
        let status = registry.register_plugin(plugin, &critic).unwrap();
        assert_eq!(status, PluginStatus::Active);

        let ctx = PluginExecutionContext::default();
        let res = registry.execute("clarify_skill", "build infinity slots", &ctx).unwrap();
        assert_eq!(res, "Instructions: Clarify the following intent: build infinity slots");
    }
}
