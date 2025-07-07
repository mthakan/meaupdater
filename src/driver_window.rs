// src/driver_window.rs

use crate::driver_manager::{self, DriverInfo, DriverType, DriverLicense};
use crate::driver_progress::DriverProgressWindow;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, ListBox, ListBoxRow, ScrolledWindow,
    Orientation, HeaderBar, Label, Dialog, MessageDialog, CheckButton,
    ButtonsType, MessageType, Separator, ComboBoxText, Switch,
    ResponseType, Frame, Expander, Grid,
};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct DriverWindow {
    window: Dialog,
}

impl DriverWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Driver Manager")
            .default_width(1600)
            .default_height(900)
            .resizable(true)
            .build();

        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("🔧 Driver Manager"))));
        window.set_titlebar(Some(&header_bar));

        Self::show_warning_dialog(&window);

        let main_vbox = GtkBox::new(Orientation::Vertical, 0);

        let control_panel = GtkBox::new(Orientation::Horizontal, 12);
        control_panel.set_margin_top(16);
        control_panel.set_margin_bottom(16);
        control_panel.set_margin_start(16);
        control_panel.set_margin_end(16);

        let filter_box = GtkBox::new(Orientation::Horizontal, 8);
        
        let filter_label = Label::new(Some("Filter:"));
        filter_label.set_markup("<b>Filter:</b>");
        filter_box.append(&filter_label);

        let type_combo = ComboBoxText::new();
        type_combo.append(Some("all"), "All Types");
        type_combo.append(Some("graphics"), "🎮 GPU");
        type_combo.append(Some("network"), "🌐 Network Cards");
        type_combo.append(Some("audio"), "🔊 Audio Cards");
        type_combo.append(Some("bluetooth"), "📡 Bluetooth");
        type_combo.append(Some("chipset"), "🔧 Chipset");
        type_combo.append(Some("other"), "🔌 Other");
        type_combo.set_active_id(Some("all"));
        filter_box.append(&type_combo);

        let license_combo = ComboBoxText::new();
        license_combo.append(Some("all"), "All Licenses");
        license_combo.append(Some("free"), "🆓 Free");
        license_combo.append(Some("nonfree"), "💰 Closed Source");
        license_combo.set_active_id(Some("all"));
        filter_box.append(&license_combo);

        let status_combo = ComboBoxText::new();
        status_combo.append(Some("all"), "All Situations");
        status_combo.append(Some("installed"), "🔵 Installed");
        status_combo.append(Some("active"), "🟢 Active");
        status_combo.append(Some("available"), "⚪ Available");
        status_combo.set_active_id(Some("all"));
        filter_box.append(&status_combo);

        control_panel.append(&filter_box);

        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk::Align::End);
        button_box.set_hexpand(true);

        let detect_btn = Button::with_label("🔍 Scan Drivers");
        detect_btn.add_css_class("header-button");
        detect_btn.add_css_class("suggested-action");

        let refresh_btn = Button::with_label("🔄 Refresh");
        refresh_btn.add_css_class("header-button");
        refresh_btn.add_css_class("refresh-button");

        let advanced_btn = Button::with_label("⚙️ Advanced");
        advanced_btn.add_css_class("header-button");

        button_box.append(&detect_btn);
        button_box.append(&refresh_btn);
        button_box.append(&advanced_btn);

        control_panel.append(&button_box);
        main_vbox.append(&control_panel);

        let separator = Separator::new(Orientation::Horizontal);
        main_vbox.append(&separator);

        let main_paned = gtk::Paned::new(Orientation::Horizontal);
        main_paned.set_vexpand(true);
        main_paned.set_margin_start(16);
        main_paned.set_margin_end(16);
        main_paned.set_margin_bottom(16);

        let left_frame = Frame::new(Some("Driver Categories"));
        left_frame.set_width_request(280);
        
        let left_scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(false)
            .width_request(280)
            .build();
            
        let category_listbox = ListBox::new();
        category_listbox.set_selection_mode(gtk::SelectionMode::Single);
        left_scrolled.set_child(Some(&category_listbox));
        left_frame.set_child(Some(&left_scrolled));

        let right_frame = Frame::new(Some("Driver Details"));
        
        let right_scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
            
        let driver_listbox = ListBox::new();
        driver_listbox.set_selection_mode(gtk::SelectionMode::None);
        right_scrolled.set_child(Some(&driver_listbox));
        right_frame.set_child(Some(&right_scrolled));

        main_paned.set_start_child(Some(&left_frame));
        main_paned.set_end_child(Some(&right_frame));
        main_paned.set_position(280);
        
        main_vbox.append(&main_paned);

        let content_area = window.content_area();
        content_area.append(&main_vbox);

        let drivers = Rc::new(RefCell::new(Vec::new()));
        let filtered_drivers = Rc::new(RefCell::new(Vec::new()));

        let drivers_clone = drivers.clone();
        let filtered_drivers_clone = filtered_drivers.clone();
        let driver_listbox_clone = driver_listbox.clone();
        let category_listbox_clone = category_listbox.clone();
        let window_clone = window.clone();
        detect_btn.connect_clicked(move |_| {
            Self::detect_drivers_with_progress(&window_clone, &drivers_clone, &filtered_drivers_clone, &driver_listbox_clone, &category_listbox_clone);
        });

        let drivers_refresh = drivers.clone();
        let filtered_drivers_refresh = filtered_drivers.clone();
        let driver_listbox_refresh = driver_listbox.clone();
        let category_listbox_refresh = category_listbox.clone();
        let window_refresh = window.clone();
        refresh_btn.connect_clicked(move |_| {
            Self::detect_drivers_with_progress(&window_refresh, &drivers_refresh, &filtered_drivers_refresh, &driver_listbox_refresh, &category_listbox_refresh);
        });

        let drivers_filter = drivers.clone();
        let filtered_drivers_filter = filtered_drivers.clone();
        let driver_listbox_filter = driver_listbox.clone();
        let category_listbox_filter = category_listbox.clone();
        let type_combo_filter = type_combo.clone();
        let license_combo_filter = license_combo.clone();
        let status_combo_filter = status_combo.clone();
        
        let filter_closure = move || {
            let type_filter = type_combo_filter.active_id().unwrap_or_else(|| "all".into()).to_string();
            let license_filter = license_combo_filter.active_id().unwrap_or_else(|| "all".into()).to_string();
            let status_filter = status_combo_filter.active_id().unwrap_or_else(|| "all".into()).to_string();
            
            let all_drivers = drivers_filter.borrow();
            let filtered = driver_manager::filter_drivers(&all_drivers, &type_filter, &license_filter, &status_filter);
            *filtered_drivers_filter.borrow_mut() = filtered.clone();
            
            Self::populate_category_list(&category_listbox_filter, &filtered);
            Self::populate_driver_list(&driver_listbox_filter, filtered);
        };

        type_combo.connect_changed({
            let filter_closure = filter_closure.clone();
            move |_| filter_closure()
        });

        license_combo.connect_changed({
            let filter_closure = filter_closure.clone();
            move |_| filter_closure()
        });

        status_combo.connect_changed({
            let filter_closure = filter_closure.clone();
            move |_| filter_closure()
        });

        let filtered_drivers_category = filtered_drivers.clone();
        let driver_listbox_category = driver_listbox.clone();
        category_listbox.connect_row_selected(move |_, selected_row| {
            if let Some(row) = selected_row {
                let index = row.index() as usize;
                Self::show_category_drivers(&filtered_drivers_category, &driver_listbox_category, index);
            }
        });

        let window_advanced = window.clone();
        let drivers_advanced = drivers.clone();
        let filtered_drivers_advanced = filtered_drivers.clone();
        let driver_listbox_advanced = driver_listbox.clone();
        let category_listbox_advanced = category_listbox.clone();
        advanced_btn.connect_clicked(move |_| {
            Self::show_advanced_options(&window_advanced, &drivers_advanced, &filtered_drivers_advanced, &driver_listbox_advanced, &category_listbox_advanced);
        });

        Self::detect_drivers_with_progress(&window, &drivers, &filtered_drivers, &driver_listbox, &category_listbox);

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn show_warning_dialog(parent: &Dialog) {
        let warning_dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Warning)
            .buttons(ButtonsType::OkCancel)
            .text("⚠️ Driver Manager Warning")
            .secondary_text("This driver manager is still in the testing phase and may cause critical errors in your software.\n\nPlease use this wisely and do not forget to backup your important data.\n\nAre you sure you want to continue?")
            .build();

        if let Some(cancel_button) = warning_dialog.widget_for_response(gtk::ResponseType::Cancel) {
            cancel_button.add_css_class("destructive-action");
        }

        let parent_clone = parent.clone();
        warning_dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Cancel {
                parent_clone.close();
            }
            dialog.close();
        });

        warning_dialog.show();
    }

    fn detect_drivers_with_progress(
        parent: &Dialog,
        drivers: &Rc<RefCell<Vec<DriverInfo>>>,
        filtered_drivers: &Rc<RefCell<Vec<DriverInfo>>>,
        driver_listbox: &ListBox,
        category_listbox: &ListBox,
    ) {
        let app_window = if let Ok(app_win) = parent.clone().upcast::<gtk::Window>().downcast::<ApplicationWindow>() {
            app_win
        } else {
            ApplicationWindow::builder().build()
        };
        
        let progress_window = DriverProgressWindow::new(&app_window);
        progress_window.set_status("Scanning drivers...");
        progress_window.set_progress(0.1, "10%");
        progress_window.append_log("Starting hardware detection...");
        progress_window.show();

        let drivers_clone = drivers.clone();
        let filtered_drivers_clone = filtered_drivers.clone();
        let driver_listbox_clone = driver_listbox.clone();
        let category_listbox_clone = category_listbox.clone();

        glib::spawn_future_local(async move {
            progress_window.set_progress(0.3, "30%");
            progress_window.append_log("Scanning PCI/USB devices...");
            
            glib::timeout_add_local(std::time::Duration::from_millis(500), {
                let progress_window = progress_window.clone();
                let drivers_clone = drivers_clone.clone();
                let filtered_drivers_clone = filtered_drivers_clone.clone();
                let driver_listbox_clone = driver_listbox_clone.clone();
                let category_listbox_clone = category_listbox_clone.clone();
                
                move || {
                    progress_window.set_progress(0.6, "60%");
                    progress_window.append_log("Checking installed drivers...");
                    
                    match driver_manager::detect_drivers() {
                        Ok(detected_drivers) => {
                            progress_window.set_progress(0.9, "90%");
                            progress_window.append_log(&format!("{} driver found", detected_drivers.len()));
                            
                            *drivers_clone.borrow_mut() = detected_drivers.clone();
                            *filtered_drivers_clone.borrow_mut() = detected_drivers.clone();
                            Self::populate_category_list(&category_listbox_clone, &detected_drivers);
                            Self::populate_driver_list(&driver_listbox_clone, detected_drivers);
                            
                            progress_window.set_progress(1.0, "100%");
                            progress_window.set_status("✅ Driver scan completed!");
                            
                            glib::timeout_add_seconds_local(2, {
                                let window = progress_window.window.clone();
                                move || {
                                    window.close();
                                    glib::ControlFlow::Break
                                }
                            });
                        }
                        Err(e) => {
                            progress_window.set_status(&format!("❌ Error: {}", e));
                            progress_window.append_log(&format!("Driver detection error: {}", e));
                        }
                    }
                    
                    glib::ControlFlow::Break
                }
            });
        });
    }

    fn populate_category_list(listbox: &ListBox, drivers: &[DriverInfo]) {
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        let grouped = driver_manager::group_drivers_by_type(drivers.to_vec());
        let mut sorted_groups: Vec<_> = grouped.into_iter().collect();
        sorted_groups.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));
        
        for (driver_type, type_drivers) in sorted_groups {
            let row = ListBoxRow::new();
            let hbox = GtkBox::new(Orientation::Horizontal, 8);
            hbox.set_margin_top(8);
            hbox.set_margin_bottom(8);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);

            let icon_label = Label::new(Some(match driver_type {
                DriverType::Graphics => "🎮",
                DriverType::Network => "🌐",
                DriverType::Audio => "🔊",
                DriverType::Bluetooth => "📡",
                DriverType::Chipset => "🔧",
                DriverType::Storage => "💾",
                DriverType::Input => "⌨️",
                DriverType::Other => "🔌",
            }));

            let name_label = Label::new(Some(&format!("{} ({})", driver_type.display_name(), type_drivers.len())));
            name_label.set_halign(gtk::Align::Start);
            name_label.set_hexpand(true);

            hbox.append(&icon_label);
            hbox.append(&name_label);
            row.set_child(Some(&hbox));
            listbox.append(&row);
        }
    }

    fn populate_driver_list(listbox: &ListBox, drivers: Vec<DriverInfo>) {
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        if drivers.is_empty() {
            let row = ListBoxRow::new();
            let empty_label = Label::new(Some("🔍 No driver found for filter"));
            empty_label.set_margin_top(20);
            empty_label.set_margin_bottom(20);
            empty_label.set_halign(gtk::Align::Center);
            row.set_child(Some(&empty_label));
            listbox.append(&row);
            return;
        }

        for driver in drivers {
            let row = ListBoxRow::new();
            row.add_css_class("package-row");

            let main_vbox = GtkBox::new(Orientation::Vertical, 8);
            main_vbox.set_margin_top(12);
            main_vbox.set_margin_bottom(12);
            main_vbox.set_margin_start(12);
            main_vbox.set_margin_end(12);

            let top_hbox = GtkBox::new(Orientation::Horizontal, 12);

            let icon_box = GtkBox::new(Orientation::Horizontal, 4);
            
            let status_icon = Label::new(Some(driver.get_status_icon()));
            let type_icon = Label::new(Some(driver.get_type_icon()));
            let license_icon = Label::new(Some(driver.get_license_icon()));

            icon_box.append(&status_icon);
            icon_box.append(&type_icon);
            icon_box.append(&license_icon);

            let info_box = GtkBox::new(Orientation::Vertical, 4);
            info_box.set_hexpand(true);

            let name_label = Label::new(Some(&driver.name));
            name_label.set_halign(gtk::Align::Start);
            name_label.set_markup(&format!("<b>{}</b>", driver.name));

            let desc_label = Label::new(Some(&driver.description));
            desc_label.set_halign(gtk::Align::Start);
            desc_label.add_css_class("version-info");

            let details_label = Label::new(Some(&format!("{} • {} • {}", 
                driver.vendor, 
                driver.version,
                match driver.license {
                    DriverLicense::Free => "Free",
                    DriverLicense::NonFree => "Closed Source",
                    DriverLicense::Unknown => "Unknown",
                }
            )));
            details_label.set_halign(gtk::Align::Start);
            details_label.add_css_class("size-info");

            info_box.append(&name_label);
            info_box.append(&desc_label);
            info_box.append(&details_label);

            let button_box = GtkBox::new(Orientation::Horizontal, 8);
            button_box.set_halign(gtk::Align::End);
            button_box.set_valign(gtk::Align::Center);

            if driver.is_recommended {
                let recommended_label = Label::new(Some("⭐ Recommended"));
                recommended_label.add_css_class("suggested-action");
                recommended_label.set_margin_end(8);
                button_box.append(&recommended_label);
            }

            if driver.is_installed {
                if driver.is_active {
                    let active_btn = Button::with_label("🟢 Active");
                    active_btn.set_sensitive(false);
                    active_btn.add_css_class("flat");
                    button_box.append(&active_btn);
                } else {
                    let activate_btn = Button::with_label("▶️ Activate");
                    activate_btn.add_css_class("suggested-action");
                    
                    let driver_clone = driver.clone();
                    activate_btn.connect_clicked(move |btn| {
                        if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                            Self::activate_driver(&window, &driver_clone);
                        }
                    });
                    
                    button_box.append(&activate_btn);
                }

                let remove_btn = Button::with_label("🗑️ Remove");
                remove_btn.add_css_class("destructive-action");
                
                let driver_clone = driver.clone();
                remove_btn.connect_clicked(move |btn| {
                    if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                        Self::show_remove_confirmation(&window, &driver_clone);
                    }
                });
                
                button_box.append(&remove_btn);
            } else {
                let install_btn = Button::with_label("⬇️ Install");
                install_btn.add_css_class("suggested-action");
                
                let driver_clone = driver.clone();
                install_btn.connect_clicked(move |btn| {
                    if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                        Self::install_driver_with_progress(&window, &driver_clone);
                    }
                });
                
                button_box.append(&install_btn);
            }

            top_hbox.append(&icon_box);
            top_hbox.append(&info_box);
            top_hbox.append(&button_box);

            main_vbox.append(&top_hbox);
            row.set_child(Some(&main_vbox));
            listbox.append(&row);
        }
    }

    fn show_category_drivers(
        drivers: &Rc<RefCell<Vec<DriverInfo>>>,
        listbox: &ListBox,
        category_index: usize,
    ) {
        let all_drivers = drivers.borrow();
        let grouped = driver_manager::group_drivers_by_type(all_drivers.clone());
        
        let mut sorted_groups: Vec<_> = grouped.into_iter().collect();
        sorted_groups.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));
        
        if let Some((_, category_drivers)) = sorted_groups.get(category_index) {
            Self::populate_driver_list(listbox, category_drivers.clone());
        }
    }

    fn activate_driver(parent: &gtk::Window, driver: &DriverInfo) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::YesNo)
            .text("▶️ Driver Activation")
            .secondary_text(&format!(
                "Are you sure you want to enable the driver '{} ({})'?",
                driver.name, driver.description
            ))
            .build();

        let driver_clone = driver.clone();
        let parent_clone = parent.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Yes {
                let success = if driver_clone.package_name.contains("nvidia") {
                    std::process::Command::new("pkexec")
                        .args(&["modprobe", "nvidia"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else if driver_clone.package_name.contains("nouveau") {
                    std::process::Command::new("pkexec")
                        .args(&["modprobe", "nouveau"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else if driver_clone.package_name == "bluez" {
                    std::process::Command::new("pkexec")
                        .args(&["systemctl", "start", "bluetooth"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else {
                    std::process::Command::new("pkexec")
                        .args(&["modprobe", &driver_clone.package_name])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                };

                let message = if success {
                    "✅ Driver activated successfully!"
                } else {
                    "❌ Driver activation failed!"
                };

                let result_dialog = MessageDialog::builder()
                    .transient_for(&parent_clone)
                    .modal(true)
                    .message_type(if success { MessageType::Info } else { MessageType::Error })
                    .buttons(ButtonsType::Ok)
                    .text(message)
                    .build();
                result_dialog.connect_response(|dlg, _| dlg.close());
                result_dialog.show();
            }
            dialog.close();
        });

        dialog.show();
    }

    fn install_driver_with_progress(parent: &gtk::Window, driver: &DriverInfo) {
        let app_window = if let Ok(app_win) = parent.clone().downcast::<ApplicationWindow>() {
            app_win
        } else {
            ApplicationWindow::builder().build()
        };
        
        let progress_window = DriverProgressWindow::new(&app_window);
        progress_window.show();
        
        let driver_clone = driver.clone();
        
        glib::spawn_future_local(async move {
            match progress_window.install_driver_with_progress(&driver_clone.package_name).await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Driver installation error: {}", e);
                }
            }
        });
    }

    fn show_remove_confirmation(parent: &gtk::Window, driver: &DriverInfo) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Warning)
            .buttons(ButtonsType::YesNo)
            .text("🗑️ Driver Removal Confirmation")
            .secondary_text(&format!(
                "Are you sure you want to uninstall driver '{} ({})'?",
                driver.name, driver.description
            ))
            .build();

        let driver_clone = driver.clone();
        let parent_clone = parent.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Yes {
                let app_window = if let Ok(app_win) = parent_clone.clone().downcast::<ApplicationWindow>() {
                    app_win
                } else {
                    ApplicationWindow::builder().build()
                };
                
                let progress_window = DriverProgressWindow::new(&app_window);
                progress_window.show();
                
                let driver_pkg = driver_clone.package_name.clone();
                glib::spawn_future_local(async move {
                    match progress_window.remove_driver_with_progress(&driver_pkg).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Driver uninstall error: {}", e);
                        }
                    }
                });
            }
            dialog.close();
        });

        dialog.show();
    }

    fn show_advanced_options(
        parent: &Dialog,
        drivers: &Rc<RefCell<Vec<DriverInfo>>>,
        filtered_drivers: &Rc<RefCell<Vec<DriverInfo>>>,
        driver_listbox: &ListBox,
        category_listbox: &ListBox,
    ) {
        let dialog = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Advanced Driver Options")
            .default_width(500)
            .default_height(400)
            .build();

        dialog.add_button("Close", ResponseType::Close);

        let content_area = dialog.content_area();
        let main_vbox = GtkBox::new(Orientation::Vertical, 16);
        main_vbox.set_margin_top(20);
        main_vbox.set_margin_bottom(20);
        main_vbox.set_margin_start(20);
        main_vbox.set_margin_end(20);

        let detection_frame = Frame::new(Some("Hardware Detection"));
        let detection_box = GtkBox::new(Orientation::Vertical, 8);
        detection_box.set_margin_top(12);
        detection_box.set_margin_bottom(12);
        detection_box.set_margin_start(12);
        detection_box.set_margin_end(12);

        let rescan_btn = Button::with_label("🔍 Rescan Hardware");
        let drivers_rescan = drivers.clone();
        let filtered_drivers_rescan = filtered_drivers.clone();
        let driver_listbox_rescan = driver_listbox.clone();
        let category_listbox_rescan = category_listbox.clone();
        let parent_rescan = parent.clone();
        rescan_btn.connect_clicked(move |_| {
            if let Err(e) = driver_manager::rescan_hardware() {
                eprintln!("Hardware scan error: {}", e);
                
                let error_dialog = MessageDialog::builder()
                    .transient_for(&parent_rescan)
                    .modal(true)
                    .message_type(MessageType::Error)
                    .buttons(ButtonsType::Ok)
                    .text(&format!("❌ Hardware rescan error:\n{}", e))
                    .build();
                error_dialog.connect_response(|dlg, _| dlg.close());
                error_dialog.show();
            } else {
                Self::detect_drivers_with_progress(&parent_rescan, &drivers_rescan, &filtered_drivers_rescan, &driver_listbox_rescan, &category_listbox_rescan);
                
                let success_dialog = MessageDialog::builder()
                    .transient_for(&parent_rescan)
                    .modal(true)
                    .message_type(MessageType::Info)
                    .buttons(ButtonsType::Ok)
                    .text("✅ Hardware rescanned and drivers updated!")
                    .build();
                success_dialog.connect_response(|dlg, _| dlg.close());
                success_dialog.show();
            }
        });

        let modalias_btn = Button::with_label("📋 Show Modalias Information");
        modalias_btn.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                Self::show_modalias_info(&window);
            }
        });

        detection_box.append(&rescan_btn);
        detection_box.append(&modalias_btn);
        detection_frame.set_child(Some(&detection_box));

        let management_frame = Frame::new(Some("Driver Management"));
        let management_box = GtkBox::new(Orientation::Vertical, 8);
        management_box.set_margin_top(12);
        management_box.set_margin_bottom(12);
        management_box.set_margin_start(12);
        management_box.set_margin_end(12);

        let backup_switch = Switch::new();
        backup_switch.set_active(true);
        let backup_box = GtkBox::new(Orientation::Horizontal, 8);
        backup_box.append(&Label::new(Some("Take backup before changing drivers:")));
        backup_box.append(&backup_switch);

        let create_backup_btn = Button::with_label("💾 Create Manual Backup");
        let parent_backup = parent.clone();
        create_backup_btn.connect_clicked(move |_| {
            match driver_manager::create_driver_backup() {
                Ok(backup_dir) => {
                    let success_dialog = MessageDialog::builder()
                        .transient_for(&parent_backup)
                        .modal(true)
                        .message_type(MessageType::Info)
                        .buttons(ButtonsType::Ok)
                        .text(&format!("✅ Backup created successfully!\n\nLocation: {}", backup_dir))
                        .build();
                    success_dialog.connect_response(|dlg, _| dlg.close());
                    success_dialog.show();
                }
                Err(e) => {
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&parent_backup)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text(&format!("❌ Backup creation error:\n{}", e))
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                }
            }
        });

        management_box.append(&backup_box);
        management_box.append(&create_backup_btn);
        management_frame.set_child(Some(&management_box));

        let system_frame = Frame::new(Some("System Information"));
        let system_box = GtkBox::new(Orientation::Vertical, 8);
        system_box.set_margin_top(12);
        system_box.set_margin_bottom(12);
        system_box.set_margin_start(12);
        system_box.set_margin_end(12);

        let kernel_info = Label::new(Some("Kernel: Loading..."));
        kernel_info.set_halign(gtk::Align::Start);
        
        let arch_info = Label::new(Some("Architecture: Loading..."));
        arch_info.set_halign(gtk::Align::Start);

        let active_modules_info = Label::new(Some("Active Modules: Loading..."));
        active_modules_info.set_halign(gtk::Align::Start);

        if let Ok(output) = std::process::Command::new("uname").arg("-r").output() {
            let kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();
            kernel_info.set_text(&format!("Kernel: {}", kernel));
        }

        if let Ok(output) = std::process::Command::new("uname").arg("-m").output() {
            let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            arch_info.set_text(&format!("Arch: {}", arch));
        }

        if let Ok(active_modules) = driver_manager::detect_active_drivers() {
            let module_count = active_modules.len();
            active_modules_info.set_text(&format!("Active Modules: {}", module_count));
        }

        system_box.append(&kernel_info);
        system_box.append(&arch_info);
        system_box.append(&active_modules_info);
        system_frame.set_child(Some(&system_box));

        main_vbox.append(&detection_frame);
        main_vbox.append(&management_frame);
        main_vbox.append(&system_frame);
        content_area.append(&main_vbox);

        dialog.connect_response(|dialog, _| {
            dialog.close();
        });

        dialog.show();
    }

    fn show_modalias_info(parent: &gtk::Window) {
        let dialog = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Modalias Information")
            .default_width(700)
            .default_height(500)
            .build();

        dialog.add_button("Close", ResponseType::Close);

        let content_area = dialog.content_area();
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        let text_view = gtk::TextView::new();
        text_view.set_editable(false);
        text_view.set_monospace(true);
        text_view.set_wrap_mode(gtk::WrapMode::Word);

        let buffer = text_view.buffer();
        
        match driver_manager::detect_hardware() {
            Ok(modaliases) => {
                let mut text = "Detected Hardware Modaliases:\n\n".to_string();
                for (i, modalias) in modaliases.iter().enumerate() {
                    text.push_str(&format!("{}. {}\n", i + 1, modalias));
                }
                buffer.set_text(&text);
            }
            Err(e) => {
                buffer.set_text(&format!("Modalias information could not be obtained: {}", e));
            }
        }

        scrolled.set_child(Some(&text_view));
        content_area.append(&scrolled);

        dialog.connect_response(|dialog, _| {
            dialog.close();
        });

        dialog.show();
    }
}
