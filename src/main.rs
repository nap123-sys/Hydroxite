use eframe::egui;
use std::fs;
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;

struct TextEditor {
    content: String,
    file_path: Option<PathBuf>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_syntax: Option<String>,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self {
            content: String::new(),
            file_path: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_syntax: None,
        }
    }
}

impl TextEditor {
    fn detect_language(&mut self) {
        if let Some(path) = &self.file_path {
            if let Some(extension) = path.extension() {
                let extension = extension.to_str().unwrap_or("");
                self.current_syntax = self.syntax_set.find_syntax_by_extension(extension)
                    .map(|s| s.name.clone());
            }
        }
    }

    fn highlight_content(&self) -> Vec<(Style, &str)> {
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let syntax = self.current_syntax
            .as_ref()
            .and_then(|s| self.syntax_set.find_syntax_by_name(s))
            .or_else(|| self.syntax_set.find_syntax_by_extension("txt"))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, theme);
        self.content.lines()
            .flat_map(|line| {
                h.highlight_line(line, &self.syntax_set)
                    .unwrap_or_default()
            })
            .collect()
    }

    fn to_hex_string(color: egui::Color32) -> String {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    }
}

impl eframe::App for TextEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.file_path = Some(path.clone());
                        self.content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
                        self.detect_language();
                    }
                }
                if ui.button("Save").clicked() {
                    if let Some(path) = &self.file_path {
                        fs::write(path, &self.content).expect("Unable to write file");
                    } else if let Some(path) = rfd::FileDialog::new().save_file() {
                        self.file_path = Some(path.clone());
                        fs::write(&path, &self.content).expect("Unable to write file");
                        self.detect_language();
                    }
                }
            });

            ui.separator();

            // Create a text edit widget
            let text_edit = egui::TextEdit::multiline(&mut self.content)
                .desired_width(f32::INFINITY)
                .desired_rows(30)
                .font(egui::TextStyle::Monospace);

            // Add the text edit widget to the UI
            let response = ui.add(text_edit);

            // If the text has changed, update the language detection
            if response.changed() {
                self.detect_language();
            }

            // Overlay the highlighted text
            let (rect, _) = ui.allocate_exact_size(response.rect.size(), egui::Sense::hover());
            let highlighted = self.highlight_content();
            let mut highlighted_text = String::new();
            for (style, text) in highlighted {
                let color = egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                highlighted_text.push_str(&format!("{}§§{}§§", TextEditor::to_hex_string(color), text));
            }
            let galley = ui.painter().layout(
                highlighted_text,
                egui::FontId::monospace(14.0),
                egui::Color32::WHITE,
                rect.width(),
            );
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: rect.min,
                galley,
                override_text_color: None,
                underline: egui::Stroke::NONE,
                angle: 0.0,
            }));
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Simple Text Editor with Syntax Highlighting",
        options,
        Box::new(|_cc| Box::new(TextEditor::default())),
    )
}