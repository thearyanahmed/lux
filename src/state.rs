use chrono::{DateTime, Utc};
use color_eyre::eyre::{self, Ok};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{fs, path::PathBuf};

use crate::api::{Task, TaskStatus};

static CFG_DIR: &str = ".luxctl";
static STATE_FILE: &str = "state.json";

// salt used for HMAC key derivation (combined with user token)
static HMAC_SALT: &str = "luxctl-state-integrity-v1";

type HmacSha256 = Hmac<Sha256>;

/// task data cached for offline access and integrity protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTask {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub points: i32,
    #[serde(default)]
    pub points_earned: i32,
    pub status: TaskStatus,
    pub sort_order: i32,
    pub validators: Vec<String>,
}

impl CachedTask {
    /// create from API task, extracting base points from scores string
    pub fn from_api_task(task: &Task) -> Self {
        // scores format: "attempts:minutes:points|..." - take max points from first tier
        let points = task
            .scores
            .split('|')
            .next()
            .and_then(|tier| tier.split(':').nth(2))
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);

        CachedTask {
            id: task.id,
            slug: task.slug.clone(),
            title: task.title.clone(),
            points,
            points_earned: task.points_earned,
            status: task.status,
            sort_order: task.sort_order,
            validators: task.validators.clone(),
        }
    }
}

/// active lab with cached task data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveLab {
    pub slug: String,
    pub name: String,
    pub fetched_at: DateTime<Utc>,
    pub tasks: Vec<CachedTask>,
    #[serde(default = "default_workspace")]
    pub workspace: String,
    #[serde(default)]
    pub runtime: Option<String>,
}

fn default_workspace() -> String {
    ".".to_string()
}

impl ActiveLab {
    pub fn total_points(&self) -> i32 {
        self.tasks.iter().map(|t| t.points).sum()
    }

    pub fn earned_points(&self) -> i32 {
        self.tasks.iter().map(|t| t.points_earned).sum()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status.is_completed())
            .count()
    }
}

/// internal state file format (includes checksum)
#[derive(Debug, Serialize, Deserialize)]
struct StateFile {
    active_lab: Option<ActiveLab>,
    checksum: String,
}

/// lab state manager with tamper detection
#[derive(Debug)]
pub struct LabState {
    pub active_lab: Option<ActiveLab>,
}

impl LabState {
    /// create empty state
    pub fn new() -> Self {
        LabState { active_lab: None }
    }

    /// load state from disk, verifying integrity with HMAC
    /// if checksum fails, returns empty state (forces re-fetch)
    pub fn load(token: &str) -> eyre::Result<Self> {
        let path = Self::state_path()?;

        if !path.exists() {
            return Ok(LabState::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| eyre::eyre!("failed to read state file: {}", e))?;

        let state_file: StateFile = serde_json::from_str(&content)
            .map_err(|e| eyre::eyre!("failed to parse state file: {}", e))?;

        // verify checksum
        let expected = Self::compute_checksum(&state_file.active_lab, token);
        if state_file.checksum != expected {
            log::warn!("state file checksum mismatch, clearing state");
            // tampered or token changed - clear state
            let empty = LabState::new();
            empty.save(token)?;
            return Ok(empty);
        }

        Ok(LabState {
            active_lab: state_file.active_lab,
        })
    }

    /// save state to disk with HMAC checksum
    pub fn save(&self, token: &str) -> eyre::Result<()> {
        let path = Self::state_path()?;

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let checksum = Self::compute_checksum(&self.active_lab, token);
        let state_file = StateFile {
            active_lab: self.active_lab.clone(),
            checksum,
        };

        let content = serde_json::to_string_pretty(&state_file)
            .map_err(|e| eyre::eyre!("failed to serialize state: {}", e))?;

        fs::write(&path, content)?;
        log::debug!("state saved to {}", path.display());

        Ok(())
    }

    /// set active lab from API data
    pub fn set_active(
        &mut self,
        slug: &str,
        name: &str,
        tasks: &[Task],
        workspace: &str,
        runtime: Option<&str>,
    ) {
        let cached_tasks: Vec<CachedTask> = tasks.iter().map(CachedTask::from_api_task).collect();

        self.active_lab = Some(ActiveLab {
            slug: slug.to_string(),
            name: name.to_string(),
            fetched_at: Utc::now(),
            tasks: cached_tasks,
            workspace: workspace.to_string(),
            runtime: runtime.map(|s| s.to_string()),
        });
    }

    /// apply a mutation to the active lab if one exists
    fn with_active_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut ActiveLab),
    {
        if let Some(ref mut lab) = self.active_lab {
            f(lab);
        }
    }

    /// update runtime for active lab
    pub fn set_runtime(&mut self, runtime: &str) {
        self.with_active_mut(|l| l.runtime = Some(runtime.to_string()));
    }

    /// update workspace for active lab
    pub fn set_workspace(&mut self, workspace: &str) {
        self.with_active_mut(|l| l.workspace = workspace.to_string());
    }

    /// clear active lab
    pub fn clear_active(&mut self) {
        self.active_lab = None;
    }

    /// get reference to active lab
    pub fn get_active(&self) -> Option<&ActiveLab> {
        self.active_lab.as_ref()
    }

    /// update cached tasks (for refresh)
    pub fn refresh_tasks(&mut self, tasks: &[Task]) {
        self.with_active_mut(|l| {
            l.tasks = tasks.iter().map(CachedTask::from_api_task).collect();
            l.fetched_at = Utc::now();
        });
    }

    /// update a single task's status (e.g., after successful submission)
    pub fn update_task_status(&mut self, task_id: i32, new_status: TaskStatus) {
        self.with_active_mut(|l| {
            if let Some(task) = l.tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = new_status;
            }
        });
    }

    /// reset all task progress to initial state (for lab restart)
    pub fn clear_progress(&mut self) {
        self.with_active_mut(|l| {
            for task in &mut l.tasks {
                task.status = TaskStatus::ChallengeAwaits;
                task.points_earned = 0;
            }
        });
    }

    /// compute HMAC-SHA256 checksum of lab data
    /// returns empty string if HMAC creation fails (should never happen for SHA256)
    fn compute_checksum(lab: &Option<ActiveLab>, token: &str) -> String {
        // derive key from token + salt
        let key = format!("{}{}", token, HMAC_SALT);

        // HMAC-SHA256 accepts any key length, so this should never fail
        let Some(mut mac) = HmacSha256::new_from_slice(key.as_bytes()).ok() else {
            log::error!("failed to create HMAC - this should never happen");
            return String::new();
        };

        // hash the lab data as JSON
        let data = serde_json::to_string(lab).unwrap_or_default();
        mac.update(data.as_bytes());

        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    fn state_path() -> eyre::Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| eyre::eyre!("could not determine home directory"))?;

        Ok(home.join(CFG_DIR).join(STATE_FILE))
    }
}

impl Default for LabState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::TaskInputType;

    fn test_token() -> &'static str {
        "test-secret-token-123"
    }

    #[test]
    fn test_cached_task_from_api_task() {
        let api_task = Task {
            id: 1,
            uuid: String::new(),
            slug: "test-task".to_string(),
            title: "Test Task".to_string(),
            description: "Description".to_string(),
            sort_order: 1,
            input_type: TaskInputType::None,
            scores: "5:10:50|10:20:35".to_string(),
            status: TaskStatus::ChallengeAwaits,
            is_free: false,
            is_locked: false,
            is_paid: false,
            abandoned_deduction: 5,
            points_earned: 35,
            hints: vec![],
            validators: vec!["tcp_listening:int(8080)".to_string()],
            prologue: vec![],
            epilogue: vec![],
        };

        let cached = CachedTask::from_api_task(&api_task);

        assert_eq!(cached.id, 1);
        assert_eq!(cached.slug, "test-task");
        assert_eq!(cached.points, 50); // max points from first tier
        assert_eq!(cached.points_earned, 35);
        assert_eq!(cached.validators.len(), 1);
    }

    #[test]
    fn test_compute_checksum_deterministic() {
        let lab = Some(ActiveLab {
            slug: "test".to_string(),
            name: "Test Lab".to_string(),
            fetched_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .expect("valid date")
                .with_timezone(&Utc),
            tasks: vec![],
            workspace: ".".to_string(),
            runtime: None,
        });

        let checksum1 = LabState::compute_checksum(&lab, test_token());
        let checksum2 = LabState::compute_checksum(&lab, test_token());

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_changes_with_data() {
        let lab1 = Some(ActiveLab {
            slug: "test1".to_string(),
            name: "Test Lab".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
            workspace: ".".to_string(),
            runtime: None,
        });

        let lab2 = Some(ActiveLab {
            slug: "test2".to_string(),
            name: "Test Lab".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
            workspace: ".".to_string(),
            runtime: None,
        });

        let checksum1 = LabState::compute_checksum(&lab1, test_token());
        let checksum2 = LabState::compute_checksum(&lab2, test_token());

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_changes_with_token() {
        let lab = Some(ActiveLab {
            slug: "test".to_string(),
            name: "Test Lab".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
            workspace: ".".to_string(),
            runtime: None,
        });

        let checksum1 = LabState::compute_checksum(&lab, "token1");
        let checksum2 = LabState::compute_checksum(&lab, "token2");

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_active_lab_stats() {
        let lab = ActiveLab {
            slug: "test".to_string(),
            name: "Test".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![
                CachedTask {
                    id: 1,
                    slug: "t1".to_string(),
                    title: "Task 1".to_string(),
                    points: 25,
                    points_earned: 20,
                    status: TaskStatus::ChallengeCompleted,
                    sort_order: 1,
                    validators: vec![],
                },
                CachedTask {
                    id: 2,
                    slug: "t2".to_string(),
                    title: "Task 2".to_string(),
                    points: 50,
                    points_earned: 0,
                    status: TaskStatus::ChallengeAwaits,
                    sort_order: 2,
                    validators: vec![],
                },
            ],
            workspace: ".".to_string(),
            runtime: Some("go".to_string()),
        };

        assert_eq!(lab.total_points(), 75);
        assert_eq!(lab.earned_points(), 20);
        assert_eq!(lab.completed_count(), 1);
    }
}
