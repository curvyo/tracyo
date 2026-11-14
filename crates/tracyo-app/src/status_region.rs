//! The hidden status region: an AccessKit polite live region that says what changed.

use eframe::egui::{self, Id, Rect, Sense, Ui, accesskit, pos2, vec2};

use crate::app::App;
use crate::mask::Tool;

/// What was said last and what the tools and the parts were when it was said.
#[derive(Debug, Default)]
pub(crate) struct Said {
    tool: Option<Tool>,
    subtract: bool,
    parts: String,
    text: String,
}

impl Said {
    /// The text after a look at the new state: a change of the region tool or of Subtract says
    /// the toggle, a change of the parts or of the mask says the mask, else the old text stays.
    fn update(&mut self, tool: Tool, subtract: bool, parts: String) {
        let Some(old) = self.tool.replace(tool) else {
            self.subtract = subtract;
            self.parts = parts;
            return;
        };
        if tool != old {
            self.text = match tool {
                Tool::Rectangle => "Rectangle on".to_owned(),
                Tool::Lasso => "Lasso on".to_owned(),
                Tool::None => format!(
                    "{} off",
                    if old == Tool::Lasso {
                        "Lasso"
                    } else {
                        "Rectangle"
                    }
                ),
            };
        } else if subtract != self.subtract {
            self.text = format!("Subtract {}", if subtract { "on" } else { "off" });
        } else if parts != self.parts {
            self.text.clone_from(&parts);
        }
        self.subtract = subtract;
        self.parts = parts;
    }
}

impl App {
    /// Puts the live region into the accessibility tree; it draws nothing.
    pub(crate) fn status_region(&mut self, ui: &mut Ui, screen: Rect) {
        if let Some(doc) = &self.doc {
            let (tool, subtract) = (doc.mask.tool, doc.mask.subtract);
            let parts = self.announcement();
            self.said.update(tool, subtract, parts);
        } else {
            self.said = Said::default();
        }
        let id = Id::unique("status-region");
        let rect = Rect::from_min_size(pos2(screen.left(), screen.bottom() - 1.0), vec2(1.0, 1.0));
        let response = ui.interact(rect, id, Sense::hover());
        let text = self.said.text.clone();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &text));
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Status);
            node.set_live(accesskit::Live::Polite);
            node.set_label(text.clone());
        });
    }
}
