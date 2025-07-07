// src/driver_progress.rs

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

#[derive(Clone)]
pub struct DriverProgressWindow {
    pub window: Window,
    pub progress_bar: ProgressBar,
    pub log_view: TextView,
    pub log_buffer: TextBuffer,
    pub status_label: Label,
}

impl DriverProgressWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Driver Process Progress")
            .default_width(650)
            .default_height(450)
            .build();

        
        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("🔧 Driver Manager - Process Progress"))));
        window.set_titlebar(Some(&header_bar));

        let main_vbox = GtkBox::new(Orientation::Vertical, 16);
        main_vbox.set_margin_top(20);
        main_vbox.set_margin_bottom(20);
        main_vbox.set_margin_start(20);
        main_vbox.set_margin_end(20);

        
        let status_label = Label::new(Some("Getting ready..."));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_markup("<b>Getting ready...</b>");
        main_vbox.append(&status_label);

        
        let progress_bar = ProgressBar::new();
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("0%"));
        progress_bar.set_height_request(30);
        main_vbox.append(&progress_bar);

        
        let log_buffer = TextBuffer::new(None::<&gtk::TextTagTable>);
        let log_view = TextView::with_buffer(&log_buffer);
        log_view.set_editable(false);
        log_view.set_cursor_visible(false);
        log_view.set_monospace(true);
        log_view.set_wrap_mode(gtk::WrapMode::Word);

        let scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .height_request(250)
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
        
        
        let mark = self.log_buffer.create_mark(None, &end_iter, false);
        self.log_view.scroll_mark_onscreen(&mark);
    }

    
    pub async fn install_driver_with_progress(&self, driver_package: &str) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<DriverProgressMessage>();

        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let log_buffer = self.log_buffer.clone();
        let log_view = self.log_view.clone();
        let window = self.window.clone();

        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        let driver_package_clone = driver_package.to_string();

        
        thread::spawn(move || {
            let _ = tx.send(DriverProgressMessage::Status("Driver loading...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.05, "5%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log(format!("Driver pack: {}", driver_package_clone)));

            
            let _ = tx.send(DriverProgressMessage::Status("Backing up system status...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log("Creating driver backup...".to_string()));
            
            match crate::driver_manager::create_driver_backup() {
                Ok(backup_dir) => {
                    let _ = tx.send(DriverProgressMessage::Log(format!("Backup created: {}", backup_dir)));
                }
                Err(e) => {
                    let _ = tx.send(DriverProgressMessage::Log(format!("Backup warning: {}", e)));
                }
            }

            // apt update
            let _ = tx.send(DriverProgressMessage::Status("Updating package list...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.2, "20%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log("Updating package list...".to_string()));
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
                                let _ = tx.send(DriverProgressMessage::Log(line));
                            }
                        }
                    }
                    
                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(DriverProgressMessage::Progress(0.4, "40%".to_string()));
                            } else {
                                let _ = tx.send(DriverProgressMessage::Error("Failed to update package list".to_string()));
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(DriverProgressMessage::Error(format!("apt update error: {}", e)));
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(DriverProgressMessage::Error(format!("apt update initialization error: {}", e)));
                    return;
                }
            }

            
            let _ = tx.send(DriverProgressMessage::Status("Sürücü paketi yükleniyor...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.5, "50%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log(format!("Loading: {}", driver_package_clone)));

            match Command::new("pkexec")
                .args(&["apt", "install", "-y", &driver_package_clone])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut progress = 0.5;
                        
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx.send(DriverProgressMessage::Log(line.clone()));
                                
                                
                                if line.contains("Unpacking") || line.contains("Setting up") || line.contains("Processing") {
                                    progress += 0.08;
                                    if progress > 0.85 { progress = 0.85; }
                                    let percent = (progress * 100.0) as i32;
                                    let _ = tx.send(DriverProgressMessage::Progress(progress, format!("{}%", percent)));
                                }
                            }
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(DriverProgressMessage::Progress(0.9, "90%".to_string()));
                                let _ = tx.send(DriverProgressMessage::Status("Configuring the driver...".to_string()));
                                let _ = tx.send(DriverProgressMessage::Log("Loading module...".to_string()));
                                
                                
                                if driver_package_clone.contains("nvidia") {
                                    let _ = std::process::Command::new("pkexec")
                                        .args(&["modprobe", "nvidia"])
                                        .output();
                                } else if driver_package_clone.contains("amd") {
                                    let _ = std::process::Command::new("pkexec")
                                        .args(&["modprobe", "amdgpu"])
                                        .output();
                                } else if driver_package_clone.contains("realtek") {
                                    // Realtek modüllerini yükle
                                    let _ = std::process::Command::new("pkexec")
                                        .args(&["modprobe", "-a", "rtl8192eu", "rtl8821ce"])
                                        .output();
                                } else if driver_package_clone.contains("broadcom") {
                                    let _ = std::process::Command::new("pkexec")
                                        .args(&["modprobe", "wl"])
                                        .output();
                                }
                                
                                let _ = tx.send(DriverProgressMessage::Progress(1.0, "100%".to_string()));
                                let _ = tx.send(DriverProgressMessage::Status("✅ Driver successfully installed!".to_string()));
                                let _ = tx.send(DriverProgressMessage::Success);
                            } else {
                                let _ = tx.send(DriverProgressMessage::Error("Driver could not be installed".to_string()));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(DriverProgressMessage::Error(format!("Driver installation error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(DriverProgressMessage::Error(format!("Driver installation initialization error: {}", e)));
                }
            }
        });

        let is_complete = Arc::new(Mutex::new(false));
        let has_error = Arc::new(Mutex::new(false));
        let is_complete_clone = is_complete.clone();
        let has_error_clone = has_error.clone();

        
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut messages_to_process = Vec::new();
            
            if let Ok(rx_guard) = rx_clone.try_lock() {
                while let Ok(msg) = rx_guard.try_recv() {
                    messages_to_process.push(msg);
                }
            }

            for msg in messages_to_process {
                match msg {
                    DriverProgressMessage::Status(status) => {
                        status_label.set_markup(&format!("<b>{}</b>", status));
                    }
                    DriverProgressMessage::Progress(fraction, text) => {
                        progress_bar.set_fraction(fraction);
                        progress_bar.set_text(Some(&text));
                    }
                    DriverProgressMessage::Log(log) => {
                        let mut end_iter = log_buffer.end_iter();
                        log_buffer.insert(&mut end_iter, &format!("{}\n", log));
                        
                        let mark = log_buffer.create_mark(None, &end_iter, false);
                        log_view.scroll_mark_onscreen(&mark);
                    }
                    DriverProgressMessage::Error(error) => {
                        status_label.set_markup(&format!("<b><span color='red'>❌ Error: {}</span></b>", error));
                        
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Driver Installation Error:\n{}", error))
                            .build();
                        dialog.connect_response(|dlg, _| dlg.close());
                        dialog.show();
                        
                        if let Ok(mut error_guard) = has_error_clone.try_lock() {
                            *error_guard = true;
                        }
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    DriverProgressMessage::Success => {
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Driver installed successfully!\n\nIt is recommended to reboot the system for the changes to take effect.")
                            .build();
                        
                        let window_clone = window.clone();
                        dialog.connect_response(move |dlg, _| {
                            dlg.close();
                            window_clone.close();
                        });
                        dialog.show();
                        
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                }
            }
            
            glib::ControlFlow::Continue
        });

        
        loop {
            glib::MainContext::default().iteration(false);
            thread::sleep(Duration::from_millis(50));
            
            if let Ok(complete_guard) = is_complete.try_lock() {
                if *complete_guard {
                    if let Ok(error_guard) = has_error.try_lock() {
                        if *error_guard {
                            return Err(anyhow::anyhow!("An error occurred during driver installation"));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }

    
    pub async fn remove_driver_with_progress(&self, driver_package: &str) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<DriverProgressMessage>();

        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let log_buffer = self.log_buffer.clone();
        let log_view = self.log_view.clone();
        let window = self.window.clone();

        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        let driver_package_clone = driver_package.to_string();

        
        thread::spawn(move || {
            let _ = tx.send(DriverProgressMessage::Status("Driver is being removed...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.05, "5%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log(format!("Driver to be removed: {}", driver_package_clone)));

            
            let _ = tx.send(DriverProgressMessage::Status("Backing up system status...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log("Creating driver backup...".to_string()));
            
            match crate::driver_manager::create_driver_backup() {
                Ok(backup_dir) => {
                    let _ = tx.send(DriverProgressMessage::Log(format!("Backup created: {}", backup_dir)));
                }
                Err(e) => {
                    let _ = tx.send(DriverProgressMessage::Log(format!("Backup warning: {}", e)));
                }
            }

            let _ = tx.send(DriverProgressMessage::Status("Driver modules are stopped...".to_string()));
            let _ = tx.send(DriverProgressMessage::Progress(0.2, "20%".to_string()));
            let _ = tx.send(DriverProgressMessage::Log("Driver modules are stopped...".to_string()));

            
            if driver_package_clone.contains("nvidia") {
                let _ = std::process::Command::new("pkexec")
                    .args(&["modprobe", "-r", "nvidia"])
                    .output();
            } else if driver_package_clone.contains("broadcom") {
                let _ = std::process::Command::new("pkexec")
                    .args(&["modprobe", "-r", "wl"])
                    .output();
            } else if driver_package_clone.contains("realtek") {
                let _ = std::process::Command::new("pkexec")
                    .args(&["modprobe", "-r", "rtl8192eu", "rtl8821ce"])
                    .output();
            } else {
                let _ = std::process::Command::new("pkexec")
                    .args(&["modprobe", "-r", &driver_package_clone])
                    .output();
            }

            let _ = tx.send(DriverProgressMessage::Progress(0.3, "30%".to_string()));
            let _ = tx.send(DriverProgressMessage::Status("Removing the package...".to_string()));
            let _ = tx.send(DriverProgressMessage::Log("Removing the package...".to_string()));

            match Command::new("pkexec")
                .args(&["apt", "remove", "--purge", "-y", &driver_package_clone])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut progress = 0.3;
                        
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx.send(DriverProgressMessage::Log(line.clone()));
                                
                                if line.contains("Removing") || line.contains("Purging") {
                                    progress += 0.15;
                                    if progress > 0.7 { progress = 0.7; }
                                    let percent = (progress * 100.0) as i32;
                                    let _ = tx.send(DriverProgressMessage::Progress(progress, format!("{}%", percent)));
                                }
                            }
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(DriverProgressMessage::Progress(0.8, "80%".to_string()));
                                let _ = tx.send(DriverProgressMessage::Status("System is being cleaned...".to_string()));
                                let _ = tx.send(DriverProgressMessage::Log("Temp files are now being cleaned...".to_string()));
                                
                                
                                let _ = std::process::Command::new("pkexec")
                                    .args(&["apt", "autoremove", "-y"])
                                    .output();
                                    
                                let _ = std::process::Command::new("pkexec")
                                    .args(&["apt", "autoclean"])
                                    .output();
                                
                                let _ = tx.send(DriverProgressMessage::Progress(1.0, "100%".to_string()));
                                let _ = tx.send(DriverProgressMessage::Status("✅ Driver successfully removed!".to_string()));
                                let _ = tx.send(DriverProgressMessage::Log("Removal completed.".to_string()));
                                let _ = tx.send(DriverProgressMessage::Success);
                            } else {
                                let _ = tx.send(DriverProgressMessage::Error("Driver could not be removed".to_string()));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(DriverProgressMessage::Error(format!("Driver uninstall error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(DriverProgressMessage::Error(format!("Driver uninstall initialization error: {}", e)));
                }
            }
        });

        let is_complete = Arc::new(Mutex::new(false));
        let has_error = Arc::new(Mutex::new(false));
        let is_complete_clone = is_complete.clone();
        let has_error_clone = has_error.clone();

        
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut messages_to_process = Vec::new();
            
            if let Ok(rx_guard) = rx_clone.try_lock() {
                while let Ok(msg) = rx_guard.try_recv() {
                    messages_to_process.push(msg);
                }
            }

            for msg in messages_to_process {
                match msg {
                    DriverProgressMessage::Status(status) => {
                        status_label.set_markup(&format!("<b>{}</b>", status));
                    }
                    DriverProgressMessage::Progress(fraction, text) => {
                        progress_bar.set_fraction(fraction);
                        progress_bar.set_text(Some(&text));
                    }
                    DriverProgressMessage::Log(log) => {
                        let mut end_iter = log_buffer.end_iter();
                        log_buffer.insert(&mut end_iter, &format!("{}\n", log));
                        
                        let mark = log_buffer.create_mark(None, &end_iter, false);
                        log_view.scroll_mark_onscreen(&mark);
                    }
                    DriverProgressMessage::Error(error) => {
                        status_label.set_markup(&format!("<b><span color='red'>❌ Error: {}</span></b>", error));
                        
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Driver Uninstall Error:\n{}", error))
                            .build();
                        dialog.connect_response(|dlg, _| dlg.close());
                        dialog.show();
                        
                        if let Ok(mut error_guard) = has_error_clone.try_lock() {
                            *error_guard = true;
                        }
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    DriverProgressMessage::Success => {
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Driver uninstalled successfully!\n\nIt is recommended to reboot the system for the changes to take effect.")
                            .build();
                        
                        let window_clone = window.clone();
                        dialog.connect_response(move |dlg, _| {
                            dlg.close();
                            window_clone.close();
                        });
                        dialog.show();
                        
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                }
            }
            
            glib::ControlFlow::Continue
        });

        
        loop {
            glib::MainContext::default().iteration(false);
            thread::sleep(Duration::from_millis(50));
            
            if let Ok(complete_guard) = is_complete.try_lock() {
                if *complete_guard {
                    if let Ok(error_guard) = has_error.try_lock() {
                        if *error_guard {
                            return Err(anyhow::anyhow!("An error occurred while uninstalling the driver"));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum DriverProgressMessage {
    Status(String),
    Progress(f64, String),
    Log(String),
    Error(String),
    Success,
}
