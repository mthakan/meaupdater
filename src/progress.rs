// src/progress.rs
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::fs;

use anyhow::Error;

#[derive(Clone)]
pub struct ProgressWindow {
    pub window: Window,
    pub progress_bar: ProgressBar,
    pub log_view: TextView,
    pub log_buffer: TextBuffer,
    pub status_label: Label,
}


static LAST_APT_UPDATE: Mutex<Option<u64>> = Mutex::new(None);
const APT_UPDATE_CACHE_DURATION: u64 = 300;

impl ProgressWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Güncelleme İlerlemesi")
            .default_width(600)
            .default_height(400)
            .build();

        
        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("📦 Update Progress"))));
        window.set_titlebar(Some(&header_bar));

        let main_vbox = GtkBox::new(Orientation::Vertical, 12);
        main_vbox.set_margin_top(16);
        main_vbox.set_margin_bottom(16);
        main_vbox.set_margin_start(16);
        main_vbox.set_margin_end(16);

        
        let status_label = Label::new(Some("Getting ready..."));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_markup("<b>Getting ready...</b>");
        main_vbox.append(&status_label);

        
        let progress_bar = ProgressBar::new();
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("0%"));
        main_vbox.append(&progress_bar);

        
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
        
        
        let mark = self.log_buffer.create_mark(None, &end_iter, false);
        self.log_view.scroll_mark_onscreen(&mark);
    }

    
    fn needs_apt_update() -> bool {
        
        if !std::path::Path::new("/var/lib/apt/lists").exists() {
            return true;
        }

        
        if let Ok(last_update_guard) = LAST_APT_UPDATE.lock() {
            if let Some(last_update) = *last_update_guard {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                return current_time - last_update > APT_UPDATE_CACHE_DURATION;
            }
        }
        
        true
    }

    
    fn update_apt_update_time() {
        if let Ok(mut last_update_guard) = LAST_APT_UPDATE.lock() {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            *last_update_guard = Some(current_time);
        }
    }

    
    fn run_apt_update_if_needed(tx: &mpsc::Sender<ProgressMessage>) -> Result<(), String> {
        if !Self::needs_apt_update() {
            let _ = tx.send(ProgressMessage::Log("Apt cache is up to date, skipping apt update...".to_string()));
            return Ok(());
        }

        let _ = tx.send(ProgressMessage::Log("Running the apt update command...".to_string()));

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
                match child.wait() {
                    Ok(status) => {
                        if !status.success() {
                            return Err("apt-update failed.".to_string());
                        }
                        
                        Self::update_apt_update_time();
                        let _ = tx.send(ProgressMessage::Log("apt update completed successfully.".to_string()));
                        Ok(())
                    }
                    Err(e) => {
                        Err(format!("apt update error: {}", e))
                    }
                }
            }
            Err(e) => {
                Err(format!("apt update initialization error: {}", e))
            }
        }
    }

    pub async fn check_updates_with_progress(&self) -> Result<Vec<crate::model::PackageUpdate>, Error> {
        let (tx, rx) = mpsc::channel::<ProgressMessage>();
        let (result_tx, result_rx) = mpsc::channel::<Result<Vec<crate::model::PackageUpdate>, Error>>();

        
        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let log_buffer = self.log_buffer.clone();
        let log_view = self.log_view.clone();
        let window = self.window.clone();

        
        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Checking the package list...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));

            
            if let Err(e) = Self::run_apt_update_if_needed(&tx) {
                let _ = tx.send(ProgressMessage::Error(e));
                let _ = result_tx.send(Err(anyhow::anyhow!("apt update error")));
                return;
            }

            let _ = tx.send(ProgressMessage::Status("Checking for updatable packages...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.6, "60%".to_string()));
            let _ = tx.send(ProgressMessage::Log("Running apt list --upgradable command...".to_string()));

            
            match Command::new("apt")
                .args(&["list", "--upgradable"])
                .env("LANG", "C")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    let mut output_text = String::new();
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx.send(ProgressMessage::Log(line.clone()));
                                output_text.push_str(&line);
                                output_text.push('\n');
                            }
                        }
                    }

                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let _ = tx.send(ProgressMessage::Progress(0.9, "90%".to_string()));
                                let _ = tx.send(ProgressMessage::Status("Packing list is being processed...".to_string()));
                                
                                let packages = crate::apt::parse_apt_list_output(&output_text);
                                let package_count = packages.len();
                                
                                let _ = tx.send(ProgressMessage::Progress(1.0, "100%".to_string()));
                                if package_count == 0 {
                                    let _ = tx.send(ProgressMessage::Status("✅ All packages are up to date!".to_string()));
                                    let _ = tx.send(ProgressMessage::Log("The package required to be updated was not found.".to_string()));
                                } else {
                                    let _ = tx.send(ProgressMessage::Status(format!("✅ {} update found!", package_count)));
                                    let _ = tx.send(ProgressMessage::Log(format!("{} updatable package found.", package_count)));
                                }
                                let _ = tx.send(ProgressMessage::CheckComplete);
                                let _ = result_tx.send(Ok(packages));
                            } else {
                                let _ = tx.send(ProgressMessage::Error("Unable to retrieve package list.".to_string()));
                                let _ = result_tx.send(Err(anyhow::anyhow!("apt list command failed")));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(ProgressMessage::Error(format!("Command error: {}", e)));
                            let _ = result_tx.send(Err(anyhow::anyhow!("Command error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(format!("apt list initialization error: {}", e)));
                    let _ = result_tx.send(Err(anyhow::anyhow!("apt list initialization error: {}", e)));
                }
            }
        });

        
        let result_packages = Arc::new(Mutex::new(Vec::new()));
        let result_error = Arc::new(Mutex::new(None::<Error>));
        let is_complete = Arc::new(Mutex::new(false));

        let result_packages_clone = result_packages.clone();
        let result_error_clone = result_error.clone();
        let is_complete_clone = is_complete.clone();

        
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut messages_to_process = Vec::new();
            
            
            if let Ok(rx_guard) = rx_clone.try_lock() {
                while let Ok(msg) = rx_guard.try_recv() {
                    messages_to_process.push(msg);
                }
            }

            
            if let Ok(res) = result_rx.try_recv() {
                match res {
                    Ok(packages) => {
                        if let Ok(mut packages_guard) = result_packages_clone.try_lock() {
                            *packages_guard = packages;
                        }
                    }
                    Err(error) => {
                        if let Ok(mut error_guard) = result_error_clone.try_lock() {
                            *error_guard = Some(error);
                        }
                    }
                }
                if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                    *complete_guard = true;
                }
            }

            
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
                        
                        
                        let mark = log_buffer.create_mark(None, &end_iter, false);
                        log_view.scroll_mark_onscreen(&mark);
                    }
                    ProgressMessage::Error(error) => {
                        status_label.set_markup(&format!("<b><span color='red'>❌ Error: {}</span></b>", error));
                        
                        
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Update Check Error:\n{}", error))
                            .build();
                        dialog.connect_response(|dlg, _| dlg.close());
                        dialog.show();
                        
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::CheckComplete => {
                        
                        glib::timeout_add_seconds_local(2, {
                            let window = window.clone();
                            move || {
                                window.close();
                                glib::ControlFlow::Break
                            }
                        });
                        
                        return glib::ControlFlow::Break;
                    }
                    _ => {} 
                }
            }
            
            glib::ControlFlow::Continue
        });

        
        loop {
            glib::MainContext::default().iteration(false);
            thread::sleep(Duration::from_millis(50));
            
            if let Ok(complete_guard) = is_complete.try_lock() {
                if *complete_guard {
                    
                    if let Ok(error_guard) = result_error.try_lock() {
                        if let Some(error) = error_guard.as_ref() {
                            return Err(anyhow::anyhow!("{}", error));
                        }
                    }
                    
                    
                    if let Ok(packages_guard) = result_packages.try_lock() {
                        return Ok(packages_guard.clone());
                    }
                }
            }
        }
    }

    pub async fn install_packages_with_progress(&self, packages: &[String]) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<ProgressMessage>();

        
        let packages_clone = packages.to_vec();
        
        
        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let log_buffer = self.log_buffer.clone();
        let log_view = self.log_view.clone();
        let window = self.window.clone();

        
        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        
        let is_complete = Arc::new(Mutex::new(false));
        let has_error = Arc::new(Mutex::new(false));
        let is_complete_clone = is_complete.clone();
        let has_error_clone = has_error.clone();

        
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Checking the package list...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));

            
            if let Err(e) = Self::run_apt_update_if_needed(&tx) {
                let _ = tx.send(ProgressMessage::Error(e));
                return;
            }

            let _ = tx.send(ProgressMessage::Status("Installing packages...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.3, "30%".to_string()));

            
            let mut args = vec!["apt", "install", "-y"];
            for pkg in &packages_clone {
                args.push(pkg);
            }

            let _ = tx.send(ProgressMessage::Log(format!("Command: pkexec {}", args.join(" "))));

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

       
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut messages_to_process = Vec::new();
            
            
            if let Ok(rx_guard) = rx_clone.try_lock() {
                while let Ok(msg) = rx_guard.try_recv() {
                    messages_to_process.push(msg);
                }
            }

            
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
                        
                        
                        let mark = log_buffer.create_mark(None, &end_iter, false);
                        log_view.scroll_mark_onscreen(&mark);
                    }
                    ProgressMessage::Error(error) => {
                        status_label.set_markup(&format!("<b><span color='red'>❌ Error: {}</span></b>", error));
                        
                        
                        let dialog = MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Installation Error:\n{}", error))
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
                    ProgressMessage::Success => {
                        
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
                        
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::CheckComplete => {
                        
                        
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
                            return Err(anyhow::anyhow!("An error occurred during installation"));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ProgressMessage {
    Status(String),
    Progress(f64, String),
    Log(String),
    Error(String),
    Success,
    CheckComplete,
}
