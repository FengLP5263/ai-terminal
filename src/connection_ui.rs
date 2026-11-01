//! User-driven status diagnostics; never enters commands or changes CLI trust.
use crate::{
    chrome,
    model::Agent,
    status::{Activity, Status},
};
use egui::{Color32, RichText};
pub fn show(ui: &mut egui::Ui, agent: Agent, status: &Status) -> egui::Response {
    let (label, color) = match status.activity {
        Activity::Unknown => ("等待任务通知", Color32::from_rgb(235, 184, 82)),
        Activity::Working => ("进行中", Color32::from_rgb(136, 180, 246)),
        Activity::Completed => ("已完成", Color32::from_rgb(88, 191, 141)),
        Activity::Waiting => ("需要处理", Color32::from_rgb(235, 184, 82)),
        Activity::Idle => ("就绪", chrome::SECONDARY),
        Activity::Exited => ("已退出", chrome::SECONDARY),
    };
    let control = ui
        .add(
            egui::Button::new(RichText::new(label).size(12.0).color(color))
                .frame(false)
                .min_size(egui::vec2(0.0, chrome::CONTROL)),
        )
        .on_hover_text("查看状态连接");
    egui::Popup::menu(&control).show(|ui| {
        ui.set_width(300.0);
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.strong("会话状态");
            let connected=status.last_event.is_some() && status.activity!=Activity::Unknown;
            if connected {
                ui.label("已连接 Agent 的任务通知");
                ui.label("进行中、需要处理与已完成随任务通知更新。");
            } else {
                ui.label(format!("已识别 {}，尚未收到任务通知。",agent.label()));
                if agent==Agent::Codex {
                    ui.label("Codex 通常在首次提交任务时连接通知；欢迎页尚未开始任务属于正常状态。");
                    ui.label("如果提交任务后仍无变化，可用 /hooks 检查状态钩子；只有未启用时才需要审查确认。");
                } else {
                    ui.label("在当前 Claude 输入 /hooks，检查状态钩子是否加载。手动指定配置文件时，请确认其中包含状态钩子。");
                }
            }
            if ui.button("复制 /hooks").clicked() {
                ui.ctx().copy_text("/hooks".into());
                ui.close();
            }
            ui.small("仅复制查看命令；信任操作由你在 Agent 内确认。");
        });
    });
    control
}
