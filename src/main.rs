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
    Insert,
    Command,
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
    vim_command: String,
    current_dir: Option<PathBuf>,
    selected_file: Option<PathBuf>,
    expanded_folders: HashMap<PathBuf, bool>,
    context_menu: Option<(PathBuf, egui::Pos2)>,
    new_item_name: String,
    creating_new_item: Option<bool>,
    refresh_tree: bool,
    icons: HashMap<String, String>,
    show_about: bool,
    version: String,
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
            vim_command: String::new(),
            current_dir: None,
            selected_file: None,
            expanded_folders: HashMap::new(),
            context_menu: None,
            new_item_name: String::new(),
            creating_new_item: None,
            refresh_tree: false,
            icons: load_icons(),
            show_about: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

fn load_icons() -> HashMap<String, String> {
    let mut icons = HashMap::new();
    icons.insert("file".to_string(), "📄".to_string());
    icons.insert("folder".to_string(), "📁".to_string());
    icons.insert("folder-open".to_string(), "📂".to_string());
    icons.insert("refresh".to_string(), "🔄".to_string());
    println!("Loaded icons: {:?}", icons); // Debug print
    icons
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

    fn highlight_content(&self) -> Vec<(Style, String)> {
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
            if let Some(refresh_icon) = self.icons.get("refresh") {
                if ui.button(refresh_icon).clicked() {
                    self.refresh_tree = true;
                }
            } else {
                println!("Refresh icon not found!"); // Debug print
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

            ui.horizontal(|ui| {
                ui.add_space((depth * 20) as f32);
                if path.is_dir() {
                    let is_expanded = self.expanded_folders.entry(path.clone()).or_insert(false);
                    let icon_key = if *is_expanded { "folder-open" } else { "folder" };
                    if let Some(icon) = self.icons.get(icon_key) {
                        if ui.button(icon).clicked() {
                            *is_expanded = !*is_expanded;
                        }
                    } else {
                        println!("Folder icon not found: {}", icon_key); // Debug print
                    }
                    ui.label(&*name);
                    if ui.rect_contains_pointer(ui.min_rect()) && ui.input(|i| i.pointer.secondary_clicked()) {
                        if let Some(pos) = ui.ctx().pointer_hover_pos() {
                            self.context_menu = Some((path.clone(), pos));
                        }
                    }
                    if *is_expanded {
                        ui.end_row();
                        self.show_folder_contents(ui, &path, depth + 1);
                    }
                } else {
                    if let Some(file_icon) = self.icons.get("file") {
                        if ui.button(file_icon).clicked() {
                            self.selected_file = Some(path.clone());
                            self.file_path = Some(path.clone());
                            self.content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
                            self.detect_language();
                        }
                    } else {
                        println!("File icon not found!"); // Debug print
                    }
                    ui.label(&*name);
                    if ui.rect_contains_pointer(ui.min_rect()) && ui.input(|i| i.pointer.secondary_clicked()) {
                        if let Some(pos) = ui.ctx().pointer_hover_pos() {
                            self.context_menu = Some((path.clone(), pos));
                        }
                    }
                }
            });
        }
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

                let highlighted = self.highlight_content();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        let mut start = 0;
                        for (style, text) in &highlighted {
                            let end = start + text.len();
                            if end > string.len() {
                                break;
                            }
                            let color = egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                            job.append(&string[start..end], 0.0, egui::TextFormat::simple(ui.style().text_styles[&egui::TextStyle::Monospace].clone(), color));
                            start = end;
                        }
                        if start < string.len() {
                            job.append(&string[start..], 0.0, egui::TextFormat::simple(ui.style().text_styles[&egui::TextStyle::Monospace].clone(), egui::Color32::WHITE));
                        }
                        ui.fonts(|f| f.layout_job(job))
                    };

                    let text_edit = egui::TextEdit::multiline(&mut self.content)
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                        .font(egui::TextStyle::Monospace)
                        .layouter(&mut layouter);

                    // Apply Vim mode if enabled
                    if self.vim_mode {
                        match self.vim_state {
                            VimMode::Normal => {
                                ui.add(text_edit.interactive(false));
                                if ui.input(|i| i.key_pressed(egui::Key::I)) {
                                    self.vim_state = VimMode::Insert;
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Colon)) {
                                    self.vim_state = VimMode::Command;
                                    self.vim_command.clear();
                                }
                            },
                            VimMode::Insert => {
                                ui.add(text_edit);
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    self.vim_state = VimMode::Normal;
                                }
                            },
                            VimMode::Command => {
                                ui.add(text_edit.interactive(false));
                            },
                        }
                    } else {
                        ui.add(text_edit);
                    }
                });

                // Vim command line
                if self.vim_mode {
                    ui.horizontal(|ui| {
                        match self.vim_state {
                            VimMode::Normal => {
                                ui.label("-- NORMAL --");
                            },
                            VimMode::Insert => {
                                ui.label("-- INSERT --");
                            },
                            VimMode::Command => {
                                ui.label(":");
                                let response = ui.text_edit_singleline(&mut self.vim_command);
                                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    // Handle Vim command
                                    match self.vim_command.as_str() {
                                        "w" => {
                                            // Save file
                                            if let Some(path) = &self.file_path {
                                                fs::write(path, &self.content).expect("Unable to write file");
                                            }
                                        },
                                        "q" => {
                                            // Quit
                                            std::process::exit(0);
                                        },
                                        "wq" => {
                                            // Save and quit
                                            if let Some(path) = &self.file_path {
                                                fs::write(path, &self.content).expect("Unable to write file");
                                            }
                                            std::process::exit(0);
                                        },
                                        _ => {
                                            // Unknown command
                                        }
                                    }
                                    self.vim_state = VimMode::Normal;
                                    self.vim_command.clear();
                                }
                            },
                        }
                    });
                }

                // Update highlighting when text changes
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.detect_language();
                }
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
        Box::new(|_cc| Box::new(TextEditor::default())),
    )
}