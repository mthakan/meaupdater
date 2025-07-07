// src/kernel_window.rs

use crate::kernel_manager::{self, KernelInfo, KernelType};
use crate::progress::ProgressWindow;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, ListBox, ListBoxRow, ScrolledWindow,
    Orientation, HeaderBar, Label, Dialog, MessageDialog,
    ButtonsType, MessageType, Separator, Expander,
    ResponseType,
};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct KernelWindow {
    window: Dialog,
}

impl KernelWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Kernel Manager")
            .default_width(900)
            .default_height(700)
            .resizable(true)
            .build();

        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("🐧 Kernel Manager"))));
        window.set_titlebar(Some(&header_bar));

        let main_vbox = GtkBox::new(Orientation::Vertical, 0);

        let current_info_frame = gtk::Frame::new(None);
        current_info_frame.set_margin_top(16);
        current_info_frame.set_margin_bottom(16);
        current_info_frame.set_margin_start(16);
        current_info_frame.set_margin_end(16);
        
        let current_info_box = GtkBox::new(Orientation::Vertical, 8);
        current_info_box.set_margin_top(12);
        current_info_box.set_margin_bottom(12);
        current_info_box.set_margin_start(12);
        current_info_box.set_margin_end(12);
        
        let current_title = Label::new(Some("Currently used kernel:"));
        current_title.set_halign(gtk::Align::Start);
        current_title.set_markup("<b>Currently used kernel:</b>");
        
        let current_kernel_label = Label::new(Some("Loading..."));
        current_kernel_label.set_halign(gtk::Align::Start);
        current_kernel_label.set_markup("<span size='large' color='#2e7d32'>🐧 Loading...</span>");
        
        current_info_box.append(&current_title);
        current_info_box.append(&current_kernel_label);
        current_info_frame.set_child(Some(&current_info_box));
        main_vbox.append(&current_info_frame);

        let main_paned = gtk::Paned::new(Orientation::Horizontal);
        main_paned.set_vexpand(true);
        main_paned.set_margin_start(16);
        main_paned.set_margin_end(16);
        main_paned.set_margin_bottom(16);

        let left_frame = gtk::Frame::new(Some("Kernel Versions"));
        left_frame.set_width_request(200);
        
        let scrolled_left = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(false)
            .width_request(200)
            .build();
            
        let version_listbox = ListBox::new();
        version_listbox.set_selection_mode(gtk::SelectionMode::Single);
        version_listbox.add_css_class("kernel-version-list");
        scrolled_left.set_child(Some(&version_listbox));
        left_frame.set_child(Some(&scrolled_left));

        let right_frame = gtk::Frame::new(Some("Kernel Details"));
        
        let right_vbox = GtkBox::new(Orientation::Vertical, 12);
        right_vbox.set_margin_top(12);
        right_vbox.set_margin_bottom(12);
        right_vbox.set_margin_start(12);
        right_vbox.set_margin_end(12);
        
        let selected_info = GtkBox::new(Orientation::Vertical, 8);
        
        let selected_version_label = Label::new(Some("Select Kernel"));
        selected_version_label.set_halign(gtk::Align::Start);
        selected_version_label.set_markup("<span size='large'><b>Select Kernel</b></span>");
        
        let selected_status_label = Label::new(Some(""));
        selected_status_label.set_halign(gtk::Align::Start);
        
        let selected_type_label = Label::new(Some(""));
        selected_type_label.set_halign(gtk::Align::Start);
        
        let selected_size_label = Label::new(Some(""));
        selected_size_label.set_halign(gtk::Align::Start);
        
        selected_info.append(&selected_version_label);
        selected_info.append(&selected_status_label);
        selected_info.append(&selected_type_label);
        selected_info.append(&selected_size_label);
        
        right_vbox.append(&selected_info);
        
        let separator = Separator::new(Orientation::Horizontal);
        right_vbox.append(&separator);
        
        let action_box = GtkBox::new(Orientation::Vertical, 8);
        
        let install_btn = Button::with_label("🔽 Install this kernel");
        install_btn.add_css_class("suggested-action");
        install_btn.set_sensitive(false);
        
        let remove_btn = Button::with_label("🗑️ Remove this kernel");
        remove_btn.add_css_class("destructive-action");
        remove_btn.set_sensitive(false);
        
        let default_btn = Button::with_label("⭐ Make default");
        default_btn.add_css_class("kernel-default-btn");
        default_btn.set_sensitive(false);
        
        let refresh_btn = Button::with_label("🔄 Refresh list");
        refresh_btn.add_css_class("refresh-button");
        
        action_box.append(&install_btn);
        action_box.append(&remove_btn);
        action_box.append(&default_btn);
        action_box.append(&refresh_btn);
        
        right_vbox.append(&action_box);
        
        right_frame.set_child(Some(&right_vbox));
        
        main_paned.set_start_child(Some(&left_frame));
        main_paned.set_end_child(Some(&right_frame));
        main_paned.set_position(250);
        
        main_vbox.append(&main_paned);

        let content_area = window.content_area();
        content_area.append(&main_vbox);

        let kernels = Rc::new(RefCell::new(Vec::new()));
        let selected_kernel = Rc::new(RefCell::new(None::<KernelInfo>));
        let kernels_clone = kernels.clone();
        let listbox_clone = version_listbox.clone();
        let current_label_clone = current_kernel_label.clone();
        let window_clone = window.clone();
        refresh_btn.connect_clicked(move |_| {
            Self::refresh_kernels_with_progress(&window_clone, &kernels_clone, &listbox_clone, &current_label_clone);
        });

        let selected_kernel_install = selected_kernel.clone();
        let kernels_install = kernels.clone();
        let listbox_install = version_listbox.clone();
        let current_label_install = current_kernel_label.clone();
        let window_install = window.clone();
        install_btn.connect_clicked(move |_| {
            if let Some(kernel) = selected_kernel_install.borrow().as_ref() {
                let kernel_clone = kernel.clone();
                let kernels_clone = kernels_install.clone();
                let listbox_clone = listbox_install.clone();
                let current_label_clone = current_label_install.clone();
                let window_clone = window_install.clone(); 
                glib::spawn_future_local(async move {
                    let app_window = window_clone.clone().upcast::<gtk::Window>().downcast::<ApplicationWindow>().unwrap_or_else(|_| ApplicationWindow::builder().build());
                    match Self::install_kernel_with_progress_impl(crate::progress::ProgressWindow::new(&app_window), &kernel_clone).await {
                        Ok(_) => {
                            kernel_manager::set_kernel_cache(vec![]);
                            Self::refresh_kernels_with_progress(&window_clone, &kernels_clone, &listbox_clone, &current_label_clone);
                        }
                        Err(e) => {
                            eprintln!("Kernel loading error: {}", e);
                        }
                    }
                });
            }
        });

        let selected_kernel_remove = selected_kernel.clone();
        let kernels_remove = kernels.clone();
        let listbox_remove = version_listbox.clone();
        let current_label_remove = current_kernel_label.clone();
        let window_remove = window.clone();
        remove_btn.connect_clicked(move |_| {
            if let Some(kernel) = selected_kernel_remove.borrow().as_ref() {
                Self::show_modern_remove_confirmation(&window_remove.clone().upcast(), kernel, &kernels_remove, &listbox_remove);
            }
        });

        let selected_kernel_default = selected_kernel.clone();
        let window_default = window.clone();
        default_btn.connect_clicked(move |_| {
            if let Some(kernel) = selected_kernel_default.borrow().as_ref() {
                Self::set_default_kernel_action(&window_default.clone().upcast(), kernel);
            }
        });

        let selected_kernel_clone = selected_kernel.clone();
        let kernels_selection = kernels.clone();
        let selected_version_clone = selected_version_label.clone();
        let selected_status_clone = selected_status_label.clone();
        let selected_type_clone = selected_type_label.clone();
        let selected_size_clone = selected_size_label.clone();
        let install_btn_clone = install_btn.clone();
        let remove_btn_clone = remove_btn.clone();
        let default_btn_clone = default_btn.clone();
        
        version_listbox.connect_row_selected(move |_, selected_row| {
            if let Some(row) = selected_row {
                let index = row.index() as usize;
                if let Ok(kernels_guard) = kernels_selection.try_borrow() {
                    if let Some(kernel) = kernels_guard.get(index) {
                        *selected_kernel_clone.borrow_mut() = Some(kernel.clone());
                        
                        selected_version_clone.set_markup(&format!("<span size='large'><b>{}</b></span>", kernel.version));
                        
                        let status_text = if kernel.is_current {
                            "<span color='#2e7d32'><b>🟢 Current kernel</b></span>"
                        } else if kernel.is_installed {
                            "<span color='#1976d2'><b>🔵 Installed</b></span>"
                        } else {
                            "<span color='#666'><b>⚪ Not installed</b></span>"
                        };
                        selected_status_clone.set_markup(status_text);
                        
                        let type_text = match kernel.kernel_type {
                            KernelType::LTS => "<span color='#388e3c'><b>🛡️ LTS (Long Term Support)</b></span>",
                            KernelType::Mainline => "<span color='#1976d2'><b>🚀 Mainline</b></span>",
                            KernelType::Unknown => "<span color='#666'><b>❓ Unknown</b></span>",
                        };
                        selected_type_clone.set_markup(type_text);
                        
                        selected_size_clone.set_markup(&format!("<b>Size:</b> {}", kernel.size));
                        
                        install_btn_clone.set_sensitive(!kernel.is_installed && !kernel.is_current);
                        remove_btn_clone.set_sensitive(kernel.is_installed && !kernel.is_current);
                        default_btn_clone.set_sensitive(kernel.is_installed && !kernel.is_current);
                    }
                }
            } else {
                *selected_kernel_clone.borrow_mut() = None;
                selected_version_clone.set_markup("<span size='large'><b>Select kernel</b></span>");
                selected_status_clone.set_text("");
                selected_type_clone.set_text("");
                selected_size_clone.set_text("");
                install_btn_clone.set_sensitive(false);
                remove_btn_clone.set_sensitive(false);
                default_btn_clone.set_sensitive(false);
            }
        });

        
        Self::refresh_kernels_with_progress(&window, &kernels, &version_listbox, &current_kernel_label);

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn refresh_kernels_with_progress(
        parent: &Dialog,
        kernels: &Rc<RefCell<Vec<KernelInfo>>>,
        listbox: &ListBox,
        current_label: &Label,
    ) {
        
        if !kernel_manager::needs_kernel_check() {
            if let Some(cached_kernels) = kernel_manager::get_cached_kernels() {
                *kernels.borrow_mut() = cached_kernels.clone();
                Self::populate_kernel_list(listbox, cached_kernels, kernels);
                Self::update_current_kernel_label(current_label);
                return;
            }
        }

        
        let app_window = if let Ok(app_win) = parent.clone().upcast::<gtk::Window>().downcast::<ApplicationWindow>() {
            app_win
        } else {
            ApplicationWindow::builder().build()
        };
        let progress_window = ProgressWindow::new(&app_window);
        progress_window.show();

        let kernels_clone = kernels.clone();
        let listbox_clone = listbox.clone();
        let current_label_clone = current_label.clone();

        
        glib::spawn_future_local(async move {
            match Self::check_kernels_with_progress(progress_window).await {
                Ok(kernel_list) => {
                    *kernels_clone.borrow_mut() = kernel_list.clone();
                    Self::populate_kernel_list(&listbox_clone, kernel_list.clone(), &kernels_clone);
                    Self::update_current_kernel_label(&current_label_clone);
                    
                   
                    kernel_manager::set_kernel_cache(kernel_list);
                    kernel_manager::update_kernel_check_time();
                }
                Err(_) => {
                    
                    Self::populate_kernel_list(&listbox_clone, vec![], &kernels_clone);
                }
            }
        });
    }

    async fn check_kernels_with_progress(progress_window: ProgressWindow) -> Result<Vec<KernelInfo>, anyhow::Error> {
        use std::sync::{Arc, Mutex, mpsc};
        use std::thread;
        use std::time::Duration;
        use crate::progress::ProgressMessage;

        let (tx, rx) = mpsc::channel::<ProgressMessage>();
        let (result_tx, result_rx) = mpsc::channel::<Result<Vec<KernelInfo>, anyhow::Error>>();

        let progress_bar = progress_window.progress_bar.clone();
        let status_label = progress_window.status_label.clone();
        let log_buffer = progress_window.log_buffer.clone();
        let log_view = progress_window.log_view.clone();

        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Checking kernel list...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(ProgressMessage::Log("Searching for available kernels...".to_string()));

            match kernel_manager::get_available_kernels() {
                Ok(mut kernels) => {
                    let _ = tx.send(ProgressMessage::Progress(0.5, "50%".to_string()));
                    let _ = tx.send(ProgressMessage::Log(format!("{} Kernel found", kernels.len())));

                    
                    let package_names: Vec<String> = kernels.iter()
                        .map(|k| k.package_name.clone())
                        .collect();
                    
                    let _ = tx.send(ProgressMessage::Status("Getting kernel sizes...".to_string()));
                    let _ = tx.send(ProgressMessage::Progress(0.8, "80%".to_string()));
                    
                    let sizes = kernel_manager::get_kernel_sizes(&package_names);
                    
                    
                    for kernel in &mut kernels {
                        if let Some(size) = sizes.get(&kernel.package_name) {
                            kernel.size = size.clone();
                        }
                    }

                    let _ = tx.send(ProgressMessage::Progress(1.0, "100%".to_string()));
                    let _ = tx.send(ProgressMessage::Status("✅ Kernel list is ready!".to_string()));
                    let _ = tx.send(ProgressMessage::CheckComplete);
                    let _ = result_tx.send(Ok(kernels));
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(format!("Could not get kernel list: {}", e)));
                    let _ = result_tx.send(Err(anyhow::anyhow!("Kernel list error: {}", e)));
                }
            }
        });

        let result_kernels = Arc::new(Mutex::new(Vec::new()));
        let result_error = Arc::new(Mutex::new(None::<anyhow::Error>));
        let is_complete = Arc::new(Mutex::new(false));

        let result_kernels_clone = result_kernels.clone();
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
                    Ok(kernels) => {
                        if let Ok(mut kernels_guard) = result_kernels_clone.try_lock() {
                            *kernels_guard = kernels;
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
                    ProgressMessage::Error(_) => {
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::CheckComplete => {
                        glib::timeout_add_seconds_local(2, {
                            let window = progress_window.window.clone();
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
                    
                    if let Ok(kernels_guard) = result_kernels.try_lock() {
                        return Ok(kernels_guard.clone());
                    }
                }
            }
        }
    }

    fn update_current_kernel_label(label: &Label) {
        match kernel_manager::get_current_kernel() {
            Ok(current) => {
                label.set_markup(&format!("<b>🐧 Current Kernel: {}</b>", current));
            }
            Err(_) => {
                label.set_markup("<b>❌ Current Kernel: Undetermined</b>");
            }
        }
    }

    fn populate_kernel_list(listbox: &ListBox, kernels: Vec<KernelInfo>, kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>) {
        
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        if kernels.is_empty() {
            let row = ListBoxRow::new();
            let empty_label = Label::new(Some("🐧 Kernel not found"));
            empty_label.set_margin_top(20);
            empty_label.set_margin_bottom(20);
            empty_label.set_halign(gtk::Align::Center);
            row.set_child(Some(&empty_label));
            listbox.append(&row);
            return;
        }

        
        let grouped_kernels = kernel_manager::group_kernels_by_major_version(kernels.clone());
        
        
        let mut sorted_groups: Vec<_> = grouped_kernels.into_iter().collect();
        sorted_groups.sort_by(|a, b| {
            let a_parts: Vec<&str> = a.0.split('.').collect();
            let b_parts: Vec<&str> = b.0.split('.').collect();
            
            for i in 0..a_parts.len().max(b_parts.len()) {
                let a_part = a_parts.get(i).unwrap_or(&"0");
                let b_part = b_parts.get(i).unwrap_or(&"0");
                
                if let (Ok(a_num), Ok(b_num)) = (a_part.parse::<i32>(), b_part.parse::<i32>()) {
                    match b_num.cmp(&a_num) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
            std::cmp::Ordering::Equal
        });

        // Tüm kernelleri tek listede göster (Linux Mint tarzı)
        let mut all_kernels = Vec::new();
        for (_, group_kernels) in sorted_groups {
            all_kernels.extend(group_kernels);
        }

        
        *kernels_ref.borrow_mut() = all_kernels.clone();

        
        for kernel in all_kernels {
            let row = ListBoxRow::new();
            row.add_css_class("kernel-version-row");

            let hbox = GtkBox::new(Orientation::Horizontal, 8);
            hbox.set_margin_top(8);
            hbox.set_margin_bottom(8);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);

            
            let version_box = GtkBox::new(Orientation::Horizontal, 6);
            
            let status_icon = if kernel.is_current {
                "🟢"
            } else if kernel.is_installed {
                "🔵" 
            } else {
                "⚪"
            };
            
            let icon_label = Label::new(Some(status_icon));
            
            let version_label = Label::new(Some(&kernel.version));
            version_label.set_halign(gtk::Align::Start);
            if kernel.is_current {
                version_label.set_markup(&format!("<b>{}</b>", kernel.version));
            }
            
            version_box.append(&icon_label);
            version_box.append(&version_label);
            
            hbox.append(&version_box);

            row.set_child(Some(&hbox));
            listbox.append(&row);
        }
    }

    fn show_modern_remove_confirmation(
        parent: &gtk::Window, 
        kernel: &KernelInfo, 
        kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>,
        listbox: &ListBox
    ) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Warning)
            .buttons(ButtonsType::YesNo)
            .text("🗑️ Kernel Removal Confirmation")
            .secondary_text(&format!(
                "Are you sure you want to remove kernel version '{}'?\n\n\
                ⚠️ This process:\n\
                • It will completely remove the kernel package\n\
                • It will delete the relevant header files\n\
                • Clean up orphaned packages with autoremove\n\
                • It will update the GRUB menu\n\n\
                This action cannot be reversed!",
                kernel.version
            ))
            .build();

        
        if let Some(yes_button) = dialog.widget_for_response(gtk::ResponseType::Yes) {
            yes_button.add_css_class("destructive-action");
        }

        let kernel_clone = kernel.clone();
        let kernels_ref_clone = kernels_ref.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Yes {
                match kernel_manager::get_current_kernel() {
                    Ok(current_kernel) => {
                        
                        Self::remove_kernel_with_progress(&parent_clone, &kernel_clone, &kernels_ref_clone, &listbox_clone, &current_kernel);
                    }
                    Err(e) => {
                        let error_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Could not get current kernel information:\n{}", e))
                            .build();
                        error_dialog.connect_response(|dlg, _| dlg.close());
                        error_dialog.show();
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }

    fn remove_kernel_with_progress(
        parent: &gtk::Window,
        kernel: &KernelInfo,
        kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>,
        listbox: &ListBox,
        current_kernel: &str
    ) {
        
        let app_window = if let Ok(app_win) = parent.clone().upcast::<gtk::Window>().downcast::<ApplicationWindow>() {
            app_win
        } else {
            ApplicationWindow::builder().build()
        };
        
        let progress_window = ProgressWindow::new(&app_window);
        progress_window.show();
        
        let kernel_clone = kernel.clone();
        let kernels_ref_clone = kernels_ref.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        let current_kernel_clone = current_kernel.to_string();
        
        
        glib::spawn_future_local(async move {
            match Self::remove_kernel_with_progress_impl(progress_window, &kernel_clone, &current_kernel_clone).await {
                Ok(_) => {
                    //2025 mthakan
                    kernel_manager::set_kernel_cache(vec![]);
                    
                    let success_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Info)
                        .buttons(ButtonsType::Ok)
                        .text("✅ Kernel removed successfully!")
                        .secondary_text("Kernel and all related packages were cleaned from the system.")
                        .build();
                    success_dialog.connect_response(|dlg, _| dlg.close());
                    success_dialog.show();
                    
                    
                    if let Some(parent_dialog) = parent_clone.downcast_ref::<Dialog>() {
                        let current_label = Label::new(Some(""));  
                        Self::refresh_kernels_with_progress(&parent_dialog, &kernels_ref_clone, &listbox_clone, &current_label);
                    }
                }
                Err(e) => {
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text(&format!("❌ Kernel removal error:\n{}", e))
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                }
            }
        });
    }

    async fn remove_kernel_with_progress_impl(
        progress_window: ProgressWindow,
        kernel: &KernelInfo,
        current_kernel: &str
    ) -> Result<(), anyhow::Error> {
        use std::sync::{Arc, Mutex, mpsc};
        use std::thread;
        use std::time::Duration;
        use std::process::{Command, Stdio};
        use std::io::{BufRead, BufReader};
        use crate::progress::ProgressMessage;

        let (tx, rx) = mpsc::channel::<ProgressMessage>();

        let progress_bar = progress_window.progress_bar.clone();
        let status_label = progress_window.status_label.clone();
        let log_buffer = progress_window.log_buffer.clone();
        let log_view = progress_window.log_view.clone();
        let window = progress_window.window.clone();

        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        let kernel_clone = kernel.clone();
        let current_kernel_clone = current_kernel.to_string();

        
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Kernel is being removed...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(ProgressMessage::Log(format!("Removing kernel {}...", kernel_clone.version)));

            
            match Command::new("pkexec")
                .args(&["apt", "remove", "--purge", "-y", &kernel_clone.package_name])
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
                            if status.success() {
                                let _ = tx.send(ProgressMessage::Progress(0.6, "60%".to_string()));
                                let _ = tx.send(ProgressMessage::Status("Orphaned packages are being cleaned...".to_string()));
                                let _ = tx.send(ProgressMessage::Log("Cleaning up orphaned packages with autoremove...".to_string()));
                                
                                
                                match Command::new("pkexec")
                                    .args(&["apt", "autoremove", "-y"])
                                    .stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                {
                                    Ok(mut autoremove_child) => {
                                        if let Some(stdout) = autoremove_child.stdout.take() {
                                            let reader = BufReader::new(stdout);
                                            for line in reader.lines() {
                                                if let Ok(line) = line {
                                                    let _ = tx.send(ProgressMessage::Log(line));
                                                }
                                            }
                                        }
                                        
                                        match autoremove_child.wait() {
                                            Ok(autoremove_status) => {
                                                if autoremove_status.success() {
                                                    let _ = tx.send(ProgressMessage::Progress(1.0, "100%".to_string()));
                                                    let _ = tx.send(ProgressMessage::Status("✅ Kernel removed successfully!".to_string()));
                                                    let _ = tx.send(ProgressMessage::Log("Kernel and related packages completely removed.".to_string()));
                                                    let _ = tx.send(ProgressMessage::Success);
                                                } else {
                                                    let _ = tx.send(ProgressMessage::Error("Autoremove operation failed".to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                let _ = tx.send(ProgressMessage::Error(format!("Autoremove error: {}", e)));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(ProgressMessage::Error(format!("Autoremove startup error: {}", e)));
                                    }
                                }
                            } else {
                                let _ = tx.send(ProgressMessage::Error("Kernel removal failed".to_string()));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(ProgressMessage::Error(format!("Command error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(format!("Uninstall initialization error: {}", e)));
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
                        
                        if let Ok(mut error_guard) = has_error_clone.try_lock() {
                            *error_guard = true;
                        }
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::Success => {
                        
                        glib::timeout_add_seconds_local(2, {
                            let window = window.clone();
                            move || {
                                window.close();
                                glib::ControlFlow::Break
                            }
                        });
                        
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
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
                    if let Ok(error_guard) = has_error.try_lock() {
                        if *error_guard {
                            return Err(anyhow::anyhow!("An error occurred during kernel removal."));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }

    fn set_default_kernel_action(
        parent: &gtk::Window,
        kernel: &KernelInfo
    ) {
        let confirmation_dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::YesNo)
            .text("⭐ Set Default Kernel")
            .secondary_text(&format!(
                "Are you sure you want to make the Kernel '{}' version the default boot option?\n\n\
                This process will change the GRUB settings and this kernel will be used on the next reboot.",
                kernel.version
            ))
            .build();

        let kernel_clone = kernel.clone();
        let parent_clone = parent.clone();
        
        confirmation_dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Yes {
                match kernel_manager::set_default_kernel(&kernel_clone.version) {
                    Ok(_) => {
                        let success_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ The default kernel has been set!")
                            .secondary_text(&format!(
                                "Kernel '{}' is now the default boot option.The change will take effect on the next restart.",
                                kernel_clone.version
                            ))
                            .build();
                        success_dialog.connect_response(|dlg, _| dlg.close());
                        success_dialog.show();
                    }
                    Err(e) => {
                        let error_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Default kernel configuration error:\n{}", e))
                            .build();
                        error_dialog.connect_response(|dlg, _| dlg.close());
                        error_dialog.show();
                    }
                }
            }
            dialog.close();
        });

        confirmation_dialog.show();
    }

    fn get_kernel_type_display(kernels: &[KernelInfo]) -> String {
        let has_lts = kernels.iter().any(|k| k.kernel_type == KernelType::LTS);
        let has_mainline = kernels.iter().any(|k| k.kernel_type == KernelType::Mainline);
        
        match (has_lts, has_mainline) {
            (true, true) => "LTS + Mainline".to_string(),
            (true, false) => "LTS".to_string(),
            (false, true) => "Mainline".to_string(),
            (false, false) => "Mixed".to_string(),
        }
    }

    fn add_kernel_group_header(listbox: &ListBox) {
        let header_row = ListBoxRow::new();
        header_row.add_css_class("kernel-header-row");
        
        let header_box = GtkBox::new(Orientation::Horizontal, 0);
        header_box.set_margin_top(8);
        header_box.set_margin_bottom(8);
        header_box.set_margin_start(8);
        header_box.set_margin_end(8);

        let status_header = Label::new(Some("Status"));
        status_header.set_width_chars(8);
        status_header.set_halign(gtk::Align::Center);
        status_header.set_markup("<b><small>Status</small></b>");

        let version_header = Label::new(Some("Kernel Version"));
        version_header.set_hexpand(true);
        version_header.set_halign(gtk::Align::Start);
        version_header.set_markup("<b><small>Kernel Version</small></b>");

        let type_header = Label::new(Some("Type"));
        type_header.set_width_chars(10);
        type_header.set_halign(gtk::Align::Center);
        type_header.set_markup("<b><small>Type</small></b>");

        let size_header = Label::new(Some("Size"));
        size_header.set_width_chars(12);
        size_header.set_halign(gtk::Align::Center);
        size_header.set_markup("<b><small>Size</small></b>");

        let actions_header = Label::new(Some("Actions"));
        actions_header.set_width_chars(15);
        actions_header.set_halign(gtk::Align::Center);
        actions_header.set_markup("<b><small>Actions</small></b>");

        header_box.append(&status_header);
        header_box.append(&version_header);
        header_box.append(&type_header);
        header_box.append(&size_header);
        header_box.append(&actions_header);

        header_row.set_child(Some(&header_box));
        listbox.append(&header_row);
    }

    fn add_kernel_row(listbox: &ListBox, kernel: &KernelInfo, kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>) {
        let row = ListBoxRow::new();
        row.add_css_class("kernel-row");

        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        
        let status_label = if kernel.is_current {
            let label = Label::new(Some("🟢 Active"));
            label.set_markup("<b><span color='green'>🟢 Active</span></b>");
            label
        } else if kernel.is_installed {
            let label = Label::new(Some("🔵 Installed"));
            label.set_markup("<span color='blue'>🔵 Installed</span>");
            label
        } else {
            let label = Label::new(Some("⚪ Not Available"));
            label.set_markup("<span color='gray'>⚪ Not Available</span>");
            label
        };
        status_label.set_width_chars(8);
        status_label.set_halign(gtk::Align::Center);

        
        let version_label = Label::new(Some(&kernel.version));
        version_label.set_hexpand(true);
        version_label.set_halign(gtk::Align::Start);
        version_label.add_css_class("package-name");
        if kernel.is_current {
            version_label.set_markup(&format!("<b>{}</b>", kernel.version));
        }

        
        let type_label = Label::new(Some(&format!("{:?}", kernel.kernel_type)));
        type_label.set_width_chars(10);
        type_label.set_halign(gtk::Align::Center);
        match kernel.kernel_type {
            KernelType::LTS => {
                type_label.set_markup("<span color='green'><b>LTS</b></span>");
            }
            KernelType::Mainline => {
                type_label.set_markup("<span color='blue'>Mainline</span>");
            }
            KernelType::Unknown => {
                type_label.set_markup("<span color='gray'>Unknown</span>");
            }
        }

        
        let size_label = Label::new(Some(&kernel.size));
        size_label.set_width_chars(12);
        size_label.set_halign(gtk::Align::Center);
        size_label.add_css_class("size-info");

        
        let actions_box = GtkBox::new(Orientation::Horizontal, 4);
        actions_box.set_width_request(120);
        actions_box.set_halign(gtk::Align::Center);

        if kernel.is_current {
            
            let current_btn = Button::with_label("Current");
            current_btn.set_sensitive(false);
            current_btn.add_css_class("flat");
            actions_box.append(&current_btn);
        } else if kernel.is_installed {
            
            let remove_btn = Button::with_label("Remove");
            remove_btn.add_css_class("flat");
            remove_btn.add_css_class("destructive-action");
            
            let kernel_clone = kernel.clone();
            let kernels_ref_clone = kernels_ref.clone();
            let listbox_clone = listbox.clone();
            remove_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                    Self::show_modern_remove_confirmation(&window, &kernel_clone, &kernels_ref_clone, &listbox_clone);
                }
            });
            
            actions_box.append(&remove_btn);
        } else {
            
            let install_btn = Button::with_label("Install");
            install_btn.add_css_class("flat");
            install_btn.add_css_class("suggested-action");
            
            let kernel_clone = kernel.clone();
            let kernels_ref_clone = kernels_ref.clone();
            let listbox_clone = listbox.clone();
            install_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                    Self::install_kernel_with_progress(&window, &kernel_clone, &kernels_ref_clone, &listbox_clone);
                }
            });
            
            actions_box.append(&install_btn);
        }

        hbox.append(&status_label);
        hbox.append(&version_label);
        hbox.append(&type_label);
        hbox.append(&size_label);
        hbox.append(&actions_box);

        row.set_child(Some(&hbox));
        listbox.append(&row);
    }

    fn show_remove_confirmation(
        parent: &gtk::Window, 
        kernel: &KernelInfo, 
        kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>,
        listbox: &ListBox
    ) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::YesNo)
            .text("🗑️ Kernel Kaldırma Onayı")
            .secondary_text(&format!(
                "Are you sure you want to remove kernel version '{}'?\n\nThis action cannot be reversed.",
                kernel.version
            ))
            .build();

        let kernel_clone = kernel.clone();
        let kernels_ref_clone = kernels_ref.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Yes {
                match kernel_manager::get_current_kernel() {
                    Ok(current_kernel) => {
                        match kernel_manager::remove_kernel_with_autoremove(&kernel_clone.package_name, &current_kernel) {
                            Ok(_) => {
                                
                                kernel_manager::set_kernel_cache(vec![]);
                                
                                let success_dialog = MessageDialog::builder()
                                    .transient_for(&parent_clone)
                                    .modal(true)
                                    .message_type(MessageType::Info)
                                    .buttons(ButtonsType::Ok)
                                    .text("✅ Kernel removed successfully!")
                                    .build();
                                success_dialog.connect_response(|dlg, _| dlg.close());
                                success_dialog.show();
                                
                                if let Some(parent_dialog) = parent_clone.downcast_ref::<Dialog>() {
                                    let current_label = Label::new(Some(""));  // 2025 mthakan
                                    Self::refresh_kernels_with_progress(&parent_dialog, &kernels_ref_clone, &listbox_clone, &current_label);
                                }
                            }
                            Err(e) => {
                                let error_dialog = MessageDialog::builder()
                                    .transient_for(&parent_clone)
                                    .modal(true)
                                    .message_type(MessageType::Error)
                                    .buttons(ButtonsType::Ok)
                                    .text(&format!("❌ Kernel removal error:\n{}", e))
                                    .build();
                                error_dialog.connect_response(|dlg, _| dlg.close());
                                error_dialog.show();
                            }
                        }
                    }
                    Err(e) => {
                        let error_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Error)
                            .buttons(ButtonsType::Ok)
                            .text(&format!("❌ Could not get current kernel information:\n{}", e))
                            .build();
                        error_dialog.connect_response(|dlg, _| dlg.close());
                        error_dialog.show();
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }

    fn install_kernel_with_progress(
        parent: &gtk::Window,
        kernel: &KernelInfo,
        kernels_ref: &Rc<RefCell<Vec<KernelInfo>>>,
        listbox: &ListBox
    ) {

        let app_window = if let Ok(app_win) = parent.clone().upcast::<gtk::Window>().downcast::<ApplicationWindow>() {
            app_win
        } else {
            ApplicationWindow::builder().build()
        };
        
        let progress_window = ProgressWindow::new(&app_window);
        progress_window.show();
        
        let kernel_clone = kernel.clone();
        let kernels_ref_clone = kernels_ref.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        

        glib::spawn_future_local(async move {
            match Self::install_kernel_with_progress_impl(progress_window, &kernel_clone).await {
                Ok(_) => {
                    kernel_manager::set_kernel_cache(vec![]);
                    
                    let success_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Info)
                        .buttons(ButtonsType::Ok)
                        .text("✅ Kernel successfully installed!")
                        .build();
                    success_dialog.connect_response(|dlg, _| dlg.close());
                    success_dialog.show();
                    
                    if let Some(parent_dialog) = parent_clone.downcast_ref::<Dialog>() {
                        let current_label = Label::new(Some(""));  // Dummy label
                        Self::refresh_kernels_with_progress(&parent_dialog, &kernels_ref_clone, &listbox_clone, &current_label);
                    }
                }
                Err(e) => {
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text(&format!("❌ Kernel installation error:\n{}", e))
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                }
            }
        });
    }

    async fn install_kernel_with_progress_impl(
        progress_window: ProgressWindow,
        kernel: &KernelInfo
    ) -> Result<(), anyhow::Error> {
        use std::sync::{Arc, Mutex, mpsc};
        use std::thread;
        use std::time::Duration;
        use std::process::{Command, Stdio};
        use std::io::{BufRead, BufReader};
        use crate::progress::ProgressMessage;

        let (tx, rx) = mpsc::channel::<ProgressMessage>();

        let progress_bar = progress_window.progress_bar.clone();
        let status_label = progress_window.status_label.clone();
        let log_buffer = progress_window.log_buffer.clone();
        let log_view = progress_window.log_view.clone();
        let window = progress_window.window.clone();

        let rx = Arc::new(Mutex::new(rx));
        let rx_clone = rx.clone();

        let kernel_clone = kernel.clone();

        
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Status("Kernel is being installed...".to_string()));
            let _ = tx.send(ProgressMessage::Progress(0.1, "10%".to_string()));
            let _ = tx.send(ProgressMessage::Log(format!("Kernel {} is being installed...", kernel_clone.version)));
//2025 mthakan
            match Command::new("pkexec")
                .args(&["apt", "install", "-y", &kernel_clone.package_name])
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
                                let _ = tx.send(ProgressMessage::Log(line.clone()));
                                
                                if line.contains("Unpacking") || line.contains("Setting up") || line.contains("Processing") {
                                    progress += 0.1;
                                    if progress > 0.9 { progress = 0.9; }
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
                                let _ = tx.send(ProgressMessage::Status("✅ Kernel successfully loaded!".to_string()));
                                let _ = tx.send(ProgressMessage::Success);
                            } else {
                                let _ = tx.send(ProgressMessage::Error("Kernel installation failed".to_string()));
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
                        
                        if let Ok(mut error_guard) = has_error_clone.try_lock() {
                            *error_guard = true;
                        }
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    ProgressMessage::Success => {
                        glib::timeout_add_seconds_local(2, {
                            let window = window.clone();
                            move || {
                                window.close();
                                glib::ControlFlow::Break
                            }
                        });
                        
                        if let Ok(mut complete_guard) = is_complete_clone.try_lock() {
                            *complete_guard = true;
                        }
                        
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
                    if let Ok(error_guard) = has_error.try_lock() {
                        if *error_guard {
                            return Err(anyhow::anyhow!("An error occurred while loading the kernel"));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }
}
