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
                ui.label(if status.native_lifecycle {
                    crate::i18n::tr(ui.ctx(),"已连接 Agent 原生状态通知","Connected to agent native lifecycle")
                } else {
                    crate::i18n::tr(ui.ctx(),"已连接 Agent 的任务通知","Connected to agent task events")
                });
                ui.label(crate::i18n::tr(ui.ctx(),"进行中、需要处理与已完成随任务通知更新。","Working, attention, failure and completion follow task events."));
            } else {
                ui.label(match locale {crate::i18n::Locale::ZhCn=>format!("已识别 {}，尚未收到任务通知。",agent.label()),crate::i18n::Locale::En=>format!("Recognized {}. No task event received yet.",agent.label())});
                if agent==Agent::Codex {
                    ui.label(crate::i18n::tr(ui.ctx(),"Codex 原生状态支持共享 agents 总览，无需额外状态 hook。已在 Codex 0.166.0 验证。","Codex native lifecycle supports the shared agents overview without additional status hooks. Verified with Codex 0.166.0."));
                    ui.label(crate::i18n::tr(ui.ctx(),"旧版没有原生通知时，可退出后运行 codex --no-daemon 使用兼容模式，再用 /hooks 审查状态钩子。此模式没有共享 agents 入口。","For older CLIs without native events, exit and run codex --no-daemon for compatibility, then review status hooks in /hooks. This mode has no shared agents entry."));
                } else if agent == Agent::QoderCn {
                    ui.label(crate::i18n::tr(ui.ctx(),"Qoder CN 通过本次启动的 --settings 接入任务 hook。请在 CLI 中自行确认目录信任，再检查 /hooks；手动传入 --settings 时沿用你的配置。","Qoder CN uses task hooks from session-scoped --settings. Review directory trust in the CLI and check /hooks; explicit --settings preserves your configuration."));
                } else if agent == Agent::Kimi {
                    ui.label(crate::i18n::tr(ui.ctx(),"Kimi Code 2.5.0 的受管 kimi 入口启用原生生命周期通知。登录与目录信任由你在 CLI 内处理；已有进程需退出后重新启动。","The managed kimi entry enables native lifecycle events in Kimi Code 2.5.0. Review login and workspace trust in the CLI; restart existing processes to apply integration."));
                } else if agent == Agent::Trae {
                    ui.label(crate::i18n::tr(ui.ctx(),"TRAE 通过本次启动的 -c 参数接入任务 hook。请在 /hooks 中审查信任；不修改用户或项目配置。","TRAE uses invocation-scoped -c task hooks. Review trust in /hooks; user and project files are not edited."));
                } else if agent == Agent::ZCode {
                    ui.label(crate::i18n::tr(ui.ctx(),"社区 zcode-app-cli 使用会话级设置快照接入开始、工具与完成通知。仅复制无凭据的基础设置；已有 hook、自定义设置或显式 ZCODE_CLI_SETTINGS_FILE 时保留原配置并跳过接入。授权等待和失败事件受上游接口限制。","Community zcode-app-cli uses an invocation-only settings snapshot for start, tool and completion events. Only credential-free basic settings are copied. Existing hooks, custom settings or explicit ZCODE_CLI_SETTINGS_FILE preserve the original configuration and skip integration. Approval waiting and failure events are limited by upstream interfaces."));
                } else {
                    ui.label(crate::i18n::tr(ui.ctx(),"在当前 Claude 输入 /hooks，检查状态钩子是否加载。手动指定配置文件时，请确认其中包含状态钩子。","Use /hooks in Claude to check the status hooks. Custom settings files must include them."));
                }
            }
            if matches!(agent, Agent::Codex | Agent::Claude | Agent::QoderCn | Agent::Trae) {
                if ui.button(crate::i18n::tr(ui.ctx(),"复制 /hooks","Copy /hooks")).clicked() {
                    ui.ctx().copy_text("/hooks".into());
                    ui.close();
                }
                ui.small(crate::i18n::tr(ui.ctx(),"仅复制查看命令；信任操作由你在 Agent 内确认。","Copies the view command only. You review trust within the agent."));
            }
        });
    });
    control
}
