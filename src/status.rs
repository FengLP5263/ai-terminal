use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Unknown,
    Idle,
    Working,
    Waiting,
    Exited,
}

/// Only metadata crosses the bridge. Prompts, tool arguments and credentials do not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    pub event_id: String,
    pub local_id: String,
    pub generation: String,
    pub name: String,
    pub session_id: String,
    pub turn_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub notification_type: Option<String>,
    pub source: Option<String>,
    pub background_running: bool,
}

#[derive(Debug, Default)]
pub struct Status {
    pub activity: Activity,
    pub unread: bool,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub last_event: Option<String>,
    waiting_tool: Option<String>,
}

impl Status {
    /// Returns false for a different agent's events or a late event from an old turn.
    pub fn apply(&mut self, event: &HookEvent, viewing: bool) -> bool {
        if event.session_id.is_empty() {
            return false;
        }
        // /clear creates a new main conversation. Child agents may inherit env.
        if let Some(id) = &self.session_id
            && id != &event.session_id
            && (event.name != "SessionStart" || event.source.as_deref() != Some("clear"))
        {
            return false;
        }
        if event.name != "UserPromptSubmit"
            && event.turn_id.is_some()
            && self.turn_id.is_some()
            && event.turn_id != self.turn_id
        {
            return false;
        }
        self.session_id = Some(event.session_id.clone());
        self.last_event = Some(event.name.clone());
        match event.name.as_str() {
            "SessionStart" => {
                if event.source.as_deref() != Some("compact") {
                    self.activity = Activity::Idle;
                    self.turn_id = None;
                    self.waiting_tool = None;
                }
            }
            "UserPromptSubmit" => {
                self.activity = Activity::Working;
                self.unread = false;
                self.turn_id = event.turn_id.clone();
                self.waiting_tool = None;
            }
            "PermissionRequest" | "Elicitation" => {
                self.activity = Activity::Waiting;
                self.waiting_tool = event.tool_use_id.clone();
            }
            "PreToolUse" => {
                if matches!(
                    event.tool_name.as_deref(),
                    Some("AskUserQuestion" | "ExitPlanMode" | "request_user_input")
                ) {
                    self.activity = Activity::Waiting;
                    self.waiting_tool = event.tool_use_id.clone();
                } else if self.activity != Activity::Waiting {
                    self.activity = Activity::Working;
                }
            }
            "PostToolUse" | "PostToolUseFailure" | "ElicitationResult" => {
                if self.activity != Activity::Waiting
                    || self.waiting_tool.is_none()
                    || self.waiting_tool == event.tool_use_id
                {
                    self.activity = Activity::Working;
                    self.waiting_tool = None;
                }
            }
            "Notification" => {
                if matches!(
                    event.notification_type.as_deref(),
                    Some("permission_prompt" | "elicitation_dialog" | "agent_needs_input")
                ) {
                    self.activity = Activity::Waiting;
                }
            }
            "Stop" => {
                // A dev server left running is not an actively processing agent turn.
                // Autonomous wakeups report a new UserPromptSubmit when work resumes.
                self.activity = Activity::Idle;
                self.unread = !viewing;
                self.waiting_tool = None;
            }
            "Interrupt" => {
                self.activity = Activity::Idle;
                self.waiting_tool = None;
            }
            "StopFailure" => {
                self.activity = Activity::Waiting;
            }
            "SessionEnd" => {
                self.activity = Activity::Exited;
            }
            _ => return false,
        }
        true
    }

    pub fn acknowledge(&mut self) {
        self.unread = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(name: &str, turn: &str) -> HookEvent {
        HookEvent {
            invocation: None,
            process_id: None,
            event_id: uuid::Uuid::new_v4().to_string(),
            local_id: "one".into(),
            generation: "first".into(),
            name: name.into(),
            session_id: "main".into(),
            turn_id: Some(turn.into()),
            tool_name: None,
            tool_use_id: None,
            notification_type: None,
            source: None,
            background_running: false,
        }
    }
    #[test]
    fn background_completion_is_visible_without_focusing_terminal() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.apply(&event("Stop", "1"), false);
        assert_eq!(s.activity, Activity::Idle);
        assert!(s.unread);
    }
    #[test]
    fn viewing_only_acknowledges_unread_not_execution() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.acknowledge();
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn old_turn_stop_cannot_finish_new_turn() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "2"), false);
        assert!(!s.apply(&event("Stop", "1"), false));
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn child_session_does_not_overwrite_main() {
        let mut s = Status::default();
        s.apply(&event("SessionStart", "1"), false);
        let mut child = event("Stop", "1");
        child.session_id = "child".into();
        assert!(!s.apply(&child, false));
        assert_eq!(s.session_id.as_deref(), Some("main"));
    }
    #[test]
    fn parallel_tool_completion_does_not_clear_other_permission() {
        let mut s = Status::default();
        let mut wait = event("PermissionRequest", "1");
        wait.tool_use_id = Some("approval".into());
        s.apply(&wait, false);
        let mut done = event("PostToolUse", "1");
        done.tool_use_id = Some("other".into());
        s.apply(&done, false);
        assert_eq!(s.activity, Activity::Waiting);
        done.tool_use_id = Some("approval".into());
        s.apply(&done, false);
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn turn_completion_is_independent_of_background_processes() {
        let mut s = Status::default();
        let mut stop = event("Stop", "1");
        stop.background_running = true;
        s.apply(&stop, false);
        assert_eq!(s.activity, Activity::Idle);
        assert!(s.unread);
    }
    #[test]
    fn compaction_does_not_reset_busy_state() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        let mut compact = event("SessionStart", "1");
        compact.source = Some("compact".into());
        s.apply(&compact, false);
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn unrelated_notification_does_not_set_waiting() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        let mut notification = event("Notification", "1");
        notification.notification_type = Some("auth_success".into());
        s.apply(&notification, false);
        assert_eq!(s.activity, Activity::Working);
    }
}
