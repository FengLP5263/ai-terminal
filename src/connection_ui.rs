//! User-driven status diagnostics; never enters commands or changes CLI trust.
use crate::{
    chrome,
    model::Agent,
    status::{Activity, Status},
};
use egui::RichText;
pub fn show(ui: &mut egui::Ui, agent: Agent, status: &Status) -> egui::Response {
    let locale = crate::i18n::locale(ui.ctx());
    let label = crate::i18n::activity(locale, status.activity);
    let color = crate::floating::status_color(status.activity);
    let control = ui
        .add(
            egui::Button::new(RichText::new(label).size(12.0).color(color))
                .frame(false)
                .min_size(egui::vec2(0.0, chrome::CONTROL)),
        )
        .on_hover_text(crate::i18n::tr(
            ui.ctx(),
            "查看状态连接",
            "View status connection",
        ));
    egui::Popup::menu(&control).show(|ui| {
        ui.set_width(300.0);
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.strong(crate::i18n::tr(ui.ctx(),"会话状态","Session status"));
            let connected=status.last_event.is_some() && status.activity!=Activity::Unknown;
            if connected {
                ui.label(if status.codex_native {
                    crate::i18n::tr(ui.ctx(),"已连接 Codex 原生状态通知","Connected to Codex native lifecycle")
                } else {
                    crate::i18n::tr(ui.ctx(),"已连接 Agent 的任务通知","Connected to agent task events")
                });
                ui.label(crate::i18n::tr(ui.ctx(),"进行中、需要处理与已完成随任务通知更新。","Working, attention, failure and completion follow task events."));
            } else {
                ui.label(match locale {crate::i18n::Locale::ZhCn=>format!("已识别 {}，尚未收到任务通知。",agent.label()),crate::i18n::Locale::En=>format!("Recognized {}. No task event received yet.",agent.label())});
                if agent==Agent::Codex {
                    ui.label(crate::i18n::tr(ui.ctx(),"Codex 原生状态支持共享 agents 总览，无需额外状态 hook。已在 Codex 0.166.0 验证。","Codex native lifecycle supports the shared agents overview without additional status hooks. Verified with Codex 0.166.0."));
                    ui.label(crate::i18n::tr(ui.ctx(),"旧版没有原生通知时，可退出后运行 codex --no-daemon 使用兼容模式，再用 /hooks 审查状态钩子。此模式没有共享 agents 入口。","For older CLIs without native events, exit and run codex --no-daemon for compatibility, then review status hooks in /hooks. This mode has no shared agents entry."));
                } else {
                    ui.label(crate::i18n::tr(ui.ctx(),"在当前 Claude 输入 /hooks，检查状态钩子是否加载。手动指定配置文件时，请确认其中包含状态钩子。","Use /hooks in Claude to check the status hooks. Custom settings files must include them."));
                }
            }
            if ui.button(crate::i18n::tr(ui.ctx(),"复制 /hooks","Copy /hooks")).clicked() {
                ui.ctx().copy_text("/hooks".into());
                ui.close();
            }
            ui.small(crate::i18n::tr(ui.ctx(),"仅复制查看命令；信任操作由你在 Agent 内确认。","Copies the view command only. You review trust within the agent."));
        });
    });
    control
}
