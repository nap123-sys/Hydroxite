use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

enum VimMode {
    Normal,
    // Insert,
    // Command,
}

struct SplashScreen {
    show_splash: bool,
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self {
            show_splash: true,
        }
    }
}

struct TextEditor {
    content: String,
    file_path: Option<PathBuf>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_syntax: Option<String>,
    splash_screen: SplashScreen,
    vim_mode: bool,
    vim_state: VimMode,
    current_dir: Option<PathBuf>,
    selected_file: Option<PathBuf>,
    expanded_folders: HashMap<PathBuf, bool>,
    context_menu: Option<(PathBuf, egui::Pos2)>,
    new_item_name: String,
    creating_new_item: Option<bool>,
    refresh_tree: bool,
    show_about: bool,
    version: String,
    rust_icon: Option<egui::TextureHandle>,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self {
            content: String::new(),
            file_path: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_syntax: None,
            splash_screen: SplashScreen::default(),
            vim_mode: false,
            vim_state: VimMode::Normal,
            current_dir: None,
            selected_file: None,
            expanded_folders: HashMap::new(),
            context_menu: None,
            new_item_name: String::new(),
            creating_new_item: None,
            refresh_tree: false,
            show_about: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
            rust_icon: None,
        }
    }
}

impl TextEditor {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut editor = Self::default();
        editor.load_rust_icon(cc);
        editor
    }

    fn load_rust_icon(&mut self, cc: &eframe::CreationContext<'_>) {
        let rust_icon_path = PathBuf::from("Rust.png");
        if rust_icon_path.exists() {
            let image = image::open(rust_icon_path).expect("Failed to open Rust.png");
            let image_buffer = image.to_rgba8();
            let size = [image.width() as _, image.height() as _];
            let image_data = egui::ColorImage::from_rgba_unmultiplied(size, image_buffer.as_flat_samples().as_slice());
            self.rust_icon = Some(cc.egui_ctx.load_texture("rust-icon", image_data, Default::default()));
        }
    }

    fn detect_language(&mut self) {
        // Existing code for detecting language
    }

    fn highlight_content(&self) -> Vec<(Style, String)> {
        // Existing code for highlighting
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let syntax = self.current_syntax
            .as_ref()
            .and_then(|s| self.syntax_set.find_syntax_by_name(s))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, theme);
        LinesWithEndings::from(&self.content)
            .flat_map(|line| {
                h.highlight_line(line, &self.syntax_set)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(style, text)| (style, text.to_string()))
            })
            .collect()
    }

    fn show_file_tree(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("File Explorer");
            if ui.button("🔄").clicked() {
                self.refresh_tree = true;
            }
        });

        if self.refresh_tree {
            self.expanded_folders.clear();
            self.refresh_tree = false;
        }

        if let Some(dir) = &self.current_dir.clone() {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_folder_contents(ui, dir, 0);
            });
        }

        // Handle context menu
        if let Some((path, pos)) = self.context_menu.take() {
            egui::Area::new("context_menu")
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(150.0);
                        if ui.button("New File").clicked() {
                            self.creating_new_item = Some(true);
                            self.new_item_name.clear();
                            ui.close_menu();
                        }
                        if ui.button("New Folder").clicked() {
                            self.creating_new_item = Some(false);
                            self.new_item_name.clear();
                            ui.close_menu();
                        }
                        if path.is_file() {
                            if ui.button("Delete").clicked() {
                                if let Err(e) = fs::remove_file(&path) {
                                    eprintln!("Failed to delete file: {}", e);
                                }
                                ui.close_menu();
                            }
                        } else if path.is_dir() {
                            if ui.button("Delete").clicked() {
                                if let Err(e) = fs::remove_dir_all(&path) {
                                    eprintln!("Failed to delete directory: {}", e);
                                }
                                ui.close_menu();
                            }
                        }
                    });
                });
        }

        // Handle new item creation
        if let Some(is_file) = self.creating_new_item {
            self.show_new_item_dialog(ui.ctx(), is_file);
        }
    }

    fn show_folder_contents(&mut self, ui: &mut egui::Ui, path: &PathBuf, depth: usize) {
        let entries = fs::read_dir(path).unwrap_or_else(|_| panic!("Failed to read directory"));
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let is_dir = path.is_dir();

            ui.horizontal(|ui| {
                ui.add_space((depth * 20) as f32);
                let is_rust_file = path.extension().map_or(false, |ext| ext == "rs");

                // Add icon
                if is_dir {
                    ui.label(if *self.expanded_folders.get(&path).unwrap_or(&false) { "▼" } else { "▶" });
                } else if is_rust_file && self.rust_icon.is_some() {
                    let rust_icon = self.rust_icon.as_ref().unwrap();
                    ui.image(rust_icon);
                } else {
                    ui.label("  "); // Spacer for other file types
                }

                // Add button with file/folder name
                let is_selected = self.selected_file.as_ref() == Some(&path);
                if ui.add(egui::SelectableLabel::new(is_selected, name.to_string())).clicked() {
                    if is_dir {
                        let is_expanded = self.expanded_folders.entry(path.clone()).or_insert(false);
                        *is_expanded = !*is_expanded;
                    } else {
                        self.load_file(&path);
                    }
                }
            });

            if is_dir && *self.expanded_folders.get(&path).unwrap_or(&false) {
                self.show_folder_contents(ui, &path, depth + 1);
            }
        }
    }

    fn load_file(&mut self, path: &PathBuf) {
        self.selected_file = Some(path.clone());
        self.file_path = Some(path.clone());
        self.content = fs::read_to_string(path).unwrap_or_else(|_| String::new());
        self.detect_language();
        self.highlight_content(); // Ensure this is called
    }

    fn show_taskbar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New File").clicked() {
                    self.creating_new_item = Some(true);
                    self.new_item_name.clear();
                    ui.close_menu();
                }
                if ui.button("New Folder").clicked() {
                    self.creating_new_item = Some(false);
                    self.new_item_name.clear();
                    ui.close_menu();
                }
                if ui.button("New").clicked() {
                    self.content = String::new();
                    self.file_path = None;
                    self.current_syntax = None;
                    ui.close_menu();
                }
                if ui.button("Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.file_path = Some(path.clone());
                        self.current_dir = path.parent().map(|p| p.to_path_buf());
                        self.content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
                        self.detect_language();
                    }
                    ui.close_menu();
                }
                if ui.button("Open Folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.current_dir = Some(path);
                        self.expanded_folders.clear(); // Reset expanded state when opening a new folder
                    }
                    ui.close_menu();
                }
                if ui.button("Save").clicked() {
                    if let Some(path) = &self.file_path {
                        fs::write(path, &self.content).expect("Unable to write file");
                    } else if let Some(path) = rfd::FileDialog::new().save_file() {
                        self.file_path = Some(path.clone());
                        fs::write(&path, &self.content).expect("Unable to write file");
                    }
                    ui.close_menu();
                }
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Cut").clicked() {
                    // Implement cut functionality
                    ui.close_menu();
                }
                if ui.button("Copy").clicked() {
                    // Implement copy functionality
                    ui.close_menu();
                }
                if ui.button("Paste").clicked() {
                    // Implement paste functionality
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.vim_mode, "Vim Mode");
                // Add more view options here
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.show_about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn show_about_dialog(&self, ctx: &egui::Context) -> bool {
        let mut should_close = false;
        egui::Window::new("About Hydroxite")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Hydroxite");
                    ui.label(format!("Version: {}", self.version));
                    ui.label("A modern text editor built with Rust and egui");
                    ui.add_space(10.0);
                    ui.label("Created by: [Your Name]");
                    ui.label("License: [Your License]");
                    ui.add_space(10.0);
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });
            });
        !should_close
    }

    fn show_new_item_dialog(&mut self, ctx: &egui::Context, is_file: bool) {
        let title = if is_file { "New File" } else { "New Folder" };
        
        egui::Window::new(title)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_item_name);
                });
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        self.create_new_item(is_file);
                        self.creating_new_item = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.creating_new_item = None;
                    }
                });
            });
    }

    fn create_new_item(&mut self, is_file: bool) {
        if let Some(current_dir) = &self.current_dir {
            let new_path = current_dir.join(&self.new_item_name);
            if is_file {
                if let Err(e) = std::fs::File::create(&new_path) {
                    eprintln!("Failed to create file: {}", e);
                } else {
                    // Optionally, open the new file in the editor
                    self.file_path = Some(new_path.clone());
                    self.content = String::new();
                    self.detect_language();
                }
            } else {
                if let Err(e) = std::fs::create_dir(&new_path) {
                    eprintln!("Failed to create folder: {}", e);
                }
            }
            // Refresh the file tree
            self.refresh_tree = true;
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        let editor = egui::TextEdit::multiline(&mut self.content)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace);

        let response = ui.add(editor);

        if response.changed() {
            // Get the cursor position from the UI state
            if let Some(cursor_pos) = ui.input(|i| i.events.iter().find_map(|e| {
                if let egui::Event::Text(_text) = e {
                    Some(self.content.len())
                } else {
                    None
                }
            })) {
                if cursor_pos > 0 {
                    let last_char = self.content.chars().nth(cursor_pos - 1);
                    if let Some(ch) = last_char {
                        let to_insert = match ch {
                            '(' => Some(')'),
                            '[' => Some(']'),
                            '{' => Some('}'),
                            '"' => Some('"'),
                            '\'' => Some('\''),
                            _ => None,
                        };

                        if let Some(closing_char) = to_insert {
                            self.content.insert(cursor_pos, closing_char);
                            // Move the cursor back between the pair
                            ui.input_mut(|i| i.events.push(egui::Event::Text(closing_char.to_string())));
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for TextEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.splash_screen.show_splash {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("Hydroxite");
                    ui.label("A modern text editor");
                    ui.add_space(20.0);
                    
                    ui.checkbox(&mut self.vim_mode, "Enable Vim Mode");
                    
                    ui.add_space(20.0);
                    if ui.button("New File").clicked() {
                        self.content = String::new();
                        self.file_path = None;
                        self.current_syntax = None;
                        self.splash_screen.show_splash = false;
                    }
                    if ui.button("Open File").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.file_path = Some(path.clone());
                            self.content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
                            self.detect_language();
                            self.splash_screen.show_splash = false;
                        }
                    }
                });
            });
        } else {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                self.show_taskbar(ui);
            });

            egui::SidePanel::left("file_tree").show(ctx, |ui| {
                self.show_file_tree(ui);
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.separator();

                let _highlighted = self.highlight_content();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_editor(ui);

                    // Vim command line
                    if self.vim_mode {
                        ui.horizontal(|ui| {
                            match self.vim_state {
                                VimMode::Normal => {
                                    ui.label("-- NORMAL --");
                                },
                                // VimMode::Insert => {
                                //     ui.label("-- INSERT --");
                                // },
                                // VimMode::Command => {
                                //     ui.label(":");
                                //     let response = ui.text_edit_singleline(&mut self.vim_command);
                                //     if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                //         // Handle Vim command
                                //         match self.vim_command.as_str() {
                                //             "w" => {
                                //                 // Save file
                                //                 if let Some(path) = &self.file_path {
                                //                     fs::write(path, &self.content).expect("Unable to write file");
                                //                 }
                                //             },
                                //             "q" => {
                                //                 // Quit
                                //                 std::process::exit(0);
                                //             },
                                //             "wq" => {
                                //                 // Save and quit
                                //                 if let Some(path) = &self.file_path {
                                //                     fs::write(path, &self.content).expect("Unable to write file");
                                //                 }
                                //                 std::process::exit(0);
                                //             },
                                //             _ => {
                                //                 // Unknown command
                                //             }
                                //         }
                                //         self.vim_state = VimMode::Normal;
                                //         self.vim_command.clear();
                                //     }
                                // },
                            }
                        });
                    }

                    // Update highlighting when text changes
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.detect_language();
                    }
                });
            });
        }

        if self.show_about {
            self.show_about = self.show_about_dialog(ctx);
        }

        if let Some(is_file) = self.creating_new_item {
            self.show_new_item_dialog(ctx, is_file);
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Hydroxite",
        options,
        Box::new(|cc| Box::new(TextEditor::new(cc))),
    )
}