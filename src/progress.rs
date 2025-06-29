use gtk::prelude::*;
use gtk::{
    Window, ApplicationWindow, Box as GtkBox, ScrolledWindow, TextView,
    TextBuffer, Orientation, HeaderBar, ButtonsType, MessageDialog,
    MessageType, Label, ProgressBar, glib
};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use anyhow::Error;

pub struct ProgressWindow {
    window: Window,
    progress_bar: ProgressBar,
    log_view: TextView,
    log_buffer: TextBuffer,
    status_label: Label,
}

impl ProgressWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Update Progress")
            .default_width(600)
            .default_height(400)
            .build();

        // Header bar
        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("📦 Update Progress"))));
        window.set_titlebar(Some(&header_bar));

        let main_vbox = GtkBox::new(Orientation::Vertical, 12);
        main_vbox.set_margin_top(16);
        main_vbox.set_margin_bottom(16);
        main_vbox.set_margin_start(16);
        main_vbox.set_margin_end(16);

        // Status label
        let status_label = Label::new(Some("Getting ready..."));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_markup("<b>Getting ready...</b>");
        main_vbox.append(&status_label);

        // Progress bar
        let progress_bar = ProgressBar::new();
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("0%"));
        main_vbox.append(&progress_bar);

        // Log display area
        let log_buffer = TextBuffer::new(None::<&gtk::TextTagTable>);
        let log_view = TextView::with_buffer(&log_buffer);
        log_view.set_editable(false);
        log_view.set_cursor_visible(false);
        log_view.set_monospace(true);

        let scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
        scrolled_window.set_child(Some(&log_view));
        main_vbox.append(&scrolled_window);

        window.set_child(Some(&main_vbox));

        Self {
            window,
            progress_bar,
            log_view,
            log_buffer,
            status_label,
        }
    }

    pub fn show(&self) {
        self.window.show();
    }

    pub fn close(&self) {
        self.window.close();
    }

    pub fn set_progress(&self, fraction: f64, text: &str) {
        self.progress_bar.set_fraction(fraction);
        self.progress_bar.set_text(Some(text));
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_markup(&format!("<b>{}</b>", status));
    }

    pub fn append_log(&self, text: &str) {
        let mut end_iter = self.log_buffer.end_iter();
        self.log_buffer.insert(&mut end_iter, &format!("{}\n", text));
        
        // Auto scroll - Scroll to the bottom in TextView
        let mark = self.log_buffer.create_mark(None, &end_iter, false);
        self.log_view.scroll_mark_onscreen(&mark);
    }

    pub fn install_packages_with_progress(&self, packages: &[String]) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<ProgressMessage>();

        // Clone packages
        let packages_clone = packages.to_vec();
        
        // clone UI elements
        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let log_buffer = self.log_buffer.clone();
        let log_view = self.log_view.clone();
        let window = self.window.clone();

        // Let's wrap the receiver with Arc<Mutex<>>
        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        // Start background thread
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Updating package list...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(ProgressMessage::Log("Running the apt update command...".to_string()));

            // apt update
            match Command::new("pkexec")
                .args(&["apt", "update"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx.send(ProgressMessage::Log(line));
                            }
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(format!("apt update error: {}", e)));
                    return;
                }
            }

            let _ = tx.send(ProgressMessage::Status("Installing packages...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.3, "30%".to_string()));

            // Install packages
            let mut args = vec!["apt", "install", "-y"];
            for pkg in &packages_clone {
                args.push(pkg);
            }

            let _ = tx.send(ProgressMessage::Log(format!("command: pkexec {}", args.join(" "))));

            match Command::new("pkexec")
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    let total_packages = packages_clone.len() as f64;
                    let mut installed_count = 0.0;

                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx.send(ProgressMessage::Log(line.clone()));
                                
                                // Progress estimation
                                if line.contains("Setting up") || line.contains("Processing") {
                                    installed_count += 1.0;
                                    let progress = 0.3 + (installed_count / total_packages) * 0.6;
                                    let percent = (progress * 100.0) as i32;
                                    let _ = tx.send(ProgressMessage::Progress(progress, format!("{}%", percent)));
                                }
                            }
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(ProgressMessage::Progress(1.0, "100%".to_string()));
                                let _ = tx.send(ProgressMessage::Status("✅ All updates installed successfully!".to_string()));
                                let _ = tx.send(ProgressMessage::Log("Installation completed.".to_string()));
                                let _ = tx.send(ProgressMessage::Success);
                            } else {
                                let _ = tx.send(ProgressMessage::Error("Installation failed.".to_string()));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(ProgressMessage::Error(format!("Command error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(format!("Installation initialization error: {}", e)));
                }
            }
        });

        // Check messages periodically
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut messages_to_process = Vec::new();
            
            // Get all available messages
            if let Ok(rx_guard) = rx_clone.try_lock() {
                while let Ok(msg) = rx_guard.try_recv() {
                    messages_to_process.push(msg);
                }
            }

            // Process messages
            for msg in messages_to_process {
                match msg {
                    ProgressMessage::Status(status) => {
                        status_label.set_markup(&format!("<b>{}</b>", status));
                    }
                    ProgressMessage::Progress(fraction, text) => {
                        progress_bar.set_fraction(fraction);
                        progress_bar.set_text(Some(&text));
                    }
                    ProgressMessage::Log(log) => {
                        let mut end_iter = log_buffer.end_iter();
                        log_buffer.insert(&mut end_iter, &format!("{}\n", log));
                        
                        // Auto scroll
                        let mark = log_buffer.create_mark(None, &end_iter, false);
                        log_view.scroll_mark_onscreen(&mark);
                    }
                    ProgressMessage::Error(error) => {
                        status_label.set_markup(&format!("<b><span color='red'>❌ Error: {}</span></b>", error));
                        
                        // Show error dialog
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Installation Error:\n{}", error))
                            .build();
                        dialog.connect_response(|dlg, _| dlg.close());
                        dialog.show();
                        
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::Success => {
                        // Show success dialog
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Updates installed successfully!")
                            .build();
                        
                        let window_clone = window.clone();
                        dialog.connect_response(move |dlg, _| {
                            dlg.close();
                            window_clone.close();
                        });
                        dialog.show();
                        
                        return glib::ControlFlow::Break;
                    }
                }
            }
            
            glib::ControlFlow::Continue
        });

        Ok(())
    }
}

#[derive(Debug)]
enum ProgressMessage {
    Status(String),
    Progress(f64, String),
    Log(String),
    Error(String),
    Success,
}
