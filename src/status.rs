use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Unknown,
    Idle,
    Completed,
    Working,
    Failed,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_invocation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_processes: Option<Vec<u32>>,
    pub event_id: String,
    pub local_id: String,
    pub generation: String,
    pub name: String,
    pub session_id: String,
    pub turn_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub notification_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    pub source: Option<String>,
    pub background_running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Default)]
pub struct Status {
    pub activity: Activity,
    pub unread: bool,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub last_event: Option<String>,
    pub native_lifecycle: bool,
    waiting_tool: Option<String>,
    qoder_settled_tools: std::collections::VecDeque<String>,
}

impl Status {
    /// Qoder permission notifications can race automatic decisions and tool results.
    pub fn apply_qoder(&mut self, event: &HookEvent, viewing: bool) -> bool {
        if event.name == "Notification"
            && event.notification_type.as_deref() == Some("permission_prompt")
            && (matches!(
                event.permission_mode.as_deref(),
                Some(
                    "auto"
                        | "bypass_permissions"
                        | "bypassPermissions"
                        | "yolo"
                        | "dont_ask"
                        | "dontAsk"
                )
            ) || event
                .tool_use_id
                .as_ref()
                .is_some_and(|id| self.qoder_settled_tools.contains(id)))
        {
            return false;
        }
        let accepted = self.apply(event, viewing);
        if accepted && event.name == "SessionStart" && event.source.as_deref() != Some("compact") {
            self.qoder_settled_tools.clear();
        }
        if accepted
            && matches!(
                event.name.as_str(),
                "PostToolUse" | "PostToolUseFailure" | "PermissionDenied"
            )
            && let Some(id) = &event.tool_use_id
            && !self.qoder_settled_tools.contains(id)
        {
            if self.qoder_settled_tools.len() == 32 {
                self.qoder_settled_tools.pop_front();
            }
            self.qoder_settled_tools.push_back(id.clone());
        }
        accepted
    }

    /// Returns false for a different agent's events or a late event from an old turn.
    pub fn apply(&mut self, event: &HookEvent, viewing: bool) -> bool {
        // Once the foreground TUI reports native lifecycle, delayed legacy
        // hooks must not overwrite a newer state or another selected chat.
        if self.native_lifecycle {
            return false;
        }
        if event.session_id.is_empty() {
            return false;
        }
        // In-process /resume, /clear and /fork switch the foreground conversation.
        // A nested agent's startup must still not steal the parent session.
        let conversation_switch = event.name == "SessionStart"
            && matches!(
                event.source.as_deref(),
                Some("resume" | "clear" | "fork" | "new")
            );
        if let Some(id) = &self.session_id
            && id != &event.session_id
            && !conversation_switch
        {
            return false;
        }
        if !matches!(event.name.as_str(), "SessionStart" | "UserPromptSubmit")
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
                    self.unread = false;
                }
            }
            "UserPromptSubmit" => {
                self.activity = Activity::Working;
                self.unread = false;
                self.turn_id = event.turn_id.clone();
                self.waiting_tool = None;
            }
            "PermissionRequest" | "Elicitation" => {
                // These fire before other hooks or automatic review may decide.
                // Wait for a user-facing notification instead.
            }
            "PostToolBatch" => {
                self.activity = Activity::Working;
                self.waiting_tool = None;
            }
            "PermissionDenied" | "PostToolUse" | "PostToolUseFailure" | "ElicitationResult" => {
                if self.activity != Activity::Waiting
                    || self.waiting_tool.is_none()
                    || self.waiting_tool == event.tool_use_id
                {
                    self.activity = Activity::Working;
                    self.waiting_tool = None;
                }
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
            "Notification" => {
                if matches!(
                    event.notification_type.as_deref(),
                    Some(
                        "permission_prompt"
                            | "elicitation_dialog"
                            | "elicitation_url_dialog"
                            | "agent_needs_input"
                    )
                ) {
                    if self.activity != Activity::Waiting || self.waiting_tool.is_none() {
                        self.waiting_tool = event.tool_use_id.clone();
                    }
                    self.activity = Activity::Waiting;
                }
            }
            "Stop" => {
                // A dev server left running is not an actively processing agent turn.
                // Autonomous wakeups report a new UserPromptSubmit when work resumes.
                self.activity = Activity::Completed;
                self.unread = !viewing;
                self.waiting_tool = None;
            }
            "Interrupt" => {
                self.activity = Activity::Idle;
                self.waiting_tool = None;
            }
            "StopFailure" => {
                self.activity = Activity::Failed;
                self.unread = !viewing;
                self.waiting_tool = None;
            }
            "SessionEnd" => {
                if self.activity != Activity::Failed {
                    self.activity = Activity::Exited;
                }
            }
            _ => return false,
        }
        true
    }

    /// The managed Codex TUI emitted a user-facing attention notification.
    pub fn codex_attention(&mut self, question: bool) {
        if self.native_lifecycle {
            return;
        }
        // A late approval notice must not revive an already ended turn. A plan
        // question can legitimately be shown after the normal Stop hook.
        if !question
            && matches!(
                self.activity,
                Activity::Completed | Activity::Failed | Activity::Exited
            )
        {
            return;
        }
        if self.activity != Activity::Waiting {
            self.waiting_tool = None;
        }
        self.activity = Activity::Waiting;
    }

    pub fn acknowledge(&mut self) {
        self.unread = false;
    }

    pub fn codex_program(&mut self, state: egui_term::CodexProgramStatus, viewing: bool) {
        use egui_term::CodexProgramStatus as Native;
        if state == Native::Clear && !self.native_lifecycle {
            return;
        }
        self.native_lifecycle = true;
        self.last_event = Some("OSC7501".into());
        self.waiting_tool = None;
        self.turn_id = None;
        self.activity = match state {
            Native::Idle | Native::Clear => Activity::Idle,
            Native::Working => Activity::Working,
            Native::Permission | Native::Question | Native::Auth => Activity::Waiting,
            Native::Done => Activity::Completed,
            Native::Error => Activity::Failed,
        };
        if matches!(state, Native::Done | Native::Error) {
            self.unread = !viewing;
        } else if state != Native::Clear {
            self.unread = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_codex_lifecycle_preserves_unread_and_ignores_late_hooks() {
        use egui_term::CodexProgramStatus as Native;
        let mut status = Status::default();
        status.codex_program(Native::Idle, false);
        assert_eq!(status.activity, Activity::Idle);
        status.codex_program(Native::Working, false);
        status.codex_program(Native::Permission, false);
        assert_eq!(status.activity, Activity::Waiting);
        status.codex_program(Native::Working, false);
        assert_eq!(status.activity, Activity::Working);
        status.codex_program(Native::Done, false);
        assert_eq!(status.activity, Activity::Completed);
        assert!(status.unread);
        assert!(!status.apply(&event("PreToolUse", "old-turn"), false));
        status.codex_attention(true);
        assert_eq!(status.activity, Activity::Completed);
        status.acknowledge();
        assert!(!status.unread);
        status.codex_program(Native::Error, true);
        assert_eq!(status.activity, Activity::Failed);
        assert!(!status.unread);
    }
    #[test]
    fn native_codex_clear_is_not_completion_or_a_connection_handshake() {
        use egui_term::CodexProgramStatus as Native;
        let mut status = Status::default();
        status.codex_program(Native::Clear, false);
        assert!(!status.native_lifecycle);
        assert_eq!(status.activity, Activity::Unknown);
        status.codex_program(Native::Working, false);
        status.codex_program(Native::Clear, false);
        assert_eq!(status.activity, Activity::Idle);
        assert!(!status.unread);
        status.codex_program(Native::Auth, false);
        assert_eq!(status.activity, Activity::Waiting);
    }
    fn event(name: &str, turn: &str) -> HookEvent {
        HookEvent {
            invocation: None,
            process_id: None,
            parent_invocation: None,
            console_processes: None,
            event_id: uuid::Uuid::new_v4().to_string(),
            local_id: "one".into(),
            generation: "first".into(),
            name: name.into(),
            session_id: "main".into(),
            turn_id: Some(turn.into()),
            tool_name: None,
            tool_use_id: None,
            notification_type: None,
            permission_mode: None,
            source: None,
            background_running: false,
            exit_code: None,
        }
    }
    #[test]
    fn qoder_automatic_permission_checks_do_not_imply_user_attention() {
        for mode in ["auto", "bypass_permissions", "dont_ask"] {
            let mut status = Status::default();
            status.apply_qoder(&event("UserPromptSubmit", "turn"), false);
            let mut notice = event("Notification", "turn");
            notice.notification_type = Some("permission_prompt".into());
            notice.permission_mode = Some(mode.into());
            notice.tool_name = Some("Bash".into());
            notice.tool_use_id = Some("shell".into());
            assert!(!status.apply_qoder(&notice, false));
            assert_eq!(status.activity, Activity::Working);
            assert!(!status.unread);

            // The same mode must not suppress a genuine question or MCP dialog.
            let mut question = event("PreToolUse", "turn");
            question.tool_name = Some("AskUserQuestion".into());
            question.tool_use_id = Some("question".into());
            question.permission_mode = Some(mode.into());
            status.apply_qoder(&question, false);
            assert_eq!(status.activity, Activity::Waiting);
            status.apply_qoder(&notice, false);
            assert_eq!(status.activity, Activity::Waiting);
            let mut done = event("PostToolUse", "turn");
            done.tool_use_id = Some("shell".into());
            status.apply_qoder(&done, false);
            assert_eq!(status.activity, Activity::Waiting);
            done.tool_use_id = Some("question".into());
            status.apply_qoder(&done, false);
            assert_eq!(status.activity, Activity::Working);
            notice.notification_type = Some("elicitation_dialog".into());
            status.apply_qoder(&notice, false);
            assert_eq!(status.activity, Activity::Waiting);
        }
    }

    #[test]
    fn qoder_manual_permission_waits_are_correlated_and_late_notices_are_ignored() {
        let mut status = Status::default();
        status.apply_qoder(&event("UserPromptSubmit", "turn"), false);
        let mut notice = event("Notification", "turn");
        notice.notification_type = Some("permission_prompt".into());
        notice.permission_mode = Some("default".into());
        notice.tool_use_id = Some("permission".into());
        assert!(status.apply_qoder(&notice, false));
        assert_eq!(status.activity, Activity::Waiting);
        let mut done = event("PostToolUse", "turn");
        done.tool_use_id = Some("other".into());
        status.apply_qoder(&done, false);
        assert_eq!(status.activity, Activity::Waiting);
        done.tool_use_id = Some("permission".into());
        status.apply_qoder(&done, false);
        assert_eq!(status.activity, Activity::Working);
        assert!(!status.apply_qoder(&notice, false));
        status.apply_qoder(&event("Stop", "turn"), false);
        assert!(!status.apply_qoder(&notice, false));
        assert_eq!(status.activity, Activity::Completed);
        assert!(status.unread);
        for n in 0..40 {
            done.tool_use_id = Some(format!("tool-{n}"));
            status.apply_qoder(&done, false);
        }
        assert_eq!(status.qoder_settled_tools.len(), 32);

        // Other Agents keep their own explicit notification semantics.
        let mut other = Status::default();
        notice.permission_mode = Some("auto".into());
        other.apply(&notice, false);
        assert_eq!(other.activity, Activity::Waiting);
    }

    #[test]
    fn background_completion_is_visible_without_focusing_terminal() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.apply(&event("Stop", "1"), false);
        assert_eq!(s.activity, Activity::Completed);
        assert!(s.unread);
    }
    #[test]
    fn viewed_completion_remains_in_status_without_becoming_unread() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), true);
        s.apply(&event("Stop", "1"), true);
        assert_eq!(s.activity, Activity::Completed);
        assert!(!s.unread);
        s.acknowledge();
        assert_eq!(s.activity, Activity::Completed);
        s.apply(&event("UserPromptSubmit", "2"), true);
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn viewing_only_acknowledges_unread_not_execution() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.acknowledge();
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn codex_automatic_permission_review_does_not_imply_user_attention() {
        let mut s = Status::default();
        assert!(s.apply(&event("UserPromptSubmit", "1"), false));
        assert!(s.apply(&event("PermissionRequest", "1"), false));
        assert_eq!(s.activity, Activity::Working);
        s.codex_attention(false);
        assert_eq!(s.activity, Activity::Waiting);
        assert!(s.apply(&event("PostToolUse", "1"), false));
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn codex_notification_preserves_correlated_question_during_parallel_work() {
        let mut s = Status::default();
        let mut question = event("PreToolUse", "1");
        question.tool_name = Some("request_user_input".into());
        question.tool_use_id = Some("question".into());
        assert!(s.apply(&question, false));
        s.codex_attention(true);
        let mut other = event("PostToolUse", "1");
        other.tool_use_id = Some("unrelated".into());
        assert!(s.apply(&other, false));
        assert_eq!(s.activity, Activity::Waiting);
        other.tool_use_id = Some("question".into());
        assert!(s.apply(&other, false));
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn claude_waits_only_for_a_visible_permission_prompt_and_recovers_after_batch() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.apply(&event("PermissionRequest", "1"), false);
        assert_eq!(s.activity, Activity::Working);
        let mut notification = event("Notification", "1");
        notification.notification_type = Some("permission_prompt".into());
        s.apply(&notification, false);
        assert_eq!(s.activity, Activity::Waiting);
        s.apply(&event("PostToolBatch", "1"), false);
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn claude_url_elicitation_is_not_confused_with_pre_dialog_event() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.apply(&event("Elicitation", "1"), false);
        assert_eq!(s.activity, Activity::Working);
        let mut notification = event("Notification", "1");
        notification.notification_type = Some("elicitation_url_dialog".into());
        s.apply(&notification, false);
        assert_eq!(s.activity, Activity::Waiting);
        s.apply(&event("ElicitationResult", "1"), false);
        assert_eq!(s.activity, Activity::Working);
    }
    #[test]
    fn stale_approval_does_not_revive_completion_but_a_new_plan_question_can() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "1"), false);
        s.apply(&event("Stop", "1"), false);
        s.codex_attention(false);
        assert_eq!(s.activity, Activity::Completed);
        s.codex_attention(true);
        assert_eq!(s.activity, Activity::Waiting);
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
        let mut wait = event("PreToolUse", "1");
        wait.tool_name = Some("AskUserQuestion".into());
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
        assert_eq!(s.activity, Activity::Completed);
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
    fn resuming_a_conversation_reconnects_status_and_rejects_previous_turns() {
        for source in ["resume", "clear", "fork", "new"] {
            let mut s = Status::default();
            assert!(s.apply(&event("UserPromptSubmit", "old-turn"), false));
            let mut start = event("SessionStart", "new-turn");
            start.session_id = "resumed-main".into();
            start.source = Some(source.into());
            assert!(
                s.apply(&start, true),
                "{source} must change the foreground conversation"
            );
            let mut prompt = event("UserPromptSubmit", "new-turn");
            prompt.session_id = start.session_id.clone();
            assert!(s.apply(&prompt, true));
            assert_eq!(s.activity, Activity::Working);
            assert!(!s.apply(&event("Stop", "old-turn"), false));
            assert_eq!(s.activity, Activity::Working);
            prompt.name = "Stop".into();
            assert!(s.apply(&prompt, false));
            assert!(s.unread);
        }
    }
    #[test]
    fn a_child_start_cannot_replace_the_foreground_conversation() {
        let mut s = Status::default();
        s.apply(&event("UserPromptSubmit", "main-turn"), false);
        let mut child = event("SessionStart", "child-turn");
        child.source = Some("startup".into());
        child.session_id = "child".into();
        assert!(!s.apply(&child, false));
        assert_eq!(s.activity, Activity::Working);
        assert_eq!(s.session_id.as_deref(), Some("main"));
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

/// Interrupted sessions are not failed tasks.
pub fn failed_exit(code: i32) -> bool {
    code != 0 && !matches!(code, 130 | 143 | -1073741510)
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    #[test]
    fn failure_is_distinct_from_questions_and_a_recoverable_tool_error() {
        let mut state = Status::default();
        let mut event:HookEvent=serde_json::from_value(serde_json::json!({"event_id":"failure","local_id":"one","generation":"new","name":"UserPromptSubmit","session_id":"agent","turn_id":"one","background_running":false})).unwrap();
        state.apply(&event, false);
        event.name = "PostToolUseFailure".into();
        state.apply(&event, false);
        assert_eq!(state.activity, Activity::Working);
        event.name = "PermissionRequest".into();
        state.apply(&event, false);
        assert_eq!(state.activity, Activity::Working);
        event.name = "Notification".into();
        event.notification_type = Some("permission_prompt".into());
        state.apply(&event, false);
        assert_eq!(state.activity, Activity::Waiting);
        event.name = "StopFailure".into();
        state.apply(&event, false);
        assert_eq!(state.activity, Activity::Failed);
        assert!(state.unread);
        event.name = "SessionEnd".into();
        state.apply(&event, false);
        assert_eq!(state.activity, Activity::Failed);
        state.acknowledge();
        assert_eq!(state.activity, Activity::Failed);
        assert!(
            !state.unread,
            "reading a failure clears only its unread flag"
        );
        for code in [0, 130, 143, -1073741510] {
            assert!(!failed_exit(code));
        }
        assert!(failed_exit(23));
    }
}
