// src/main.rs
mod about;
mod apt;
mod model;
mod policy;
mod progress;
mod repo_manager;
mod repo_window;
mod kernel_manager;
mod kernel_window;
mod driver_manager;
mod driver_window;
mod driver_progress;

use anyhow::Error;
use gtk::prelude::*;
use gtk::{
    Application,
    ApplicationWindow,
    Box as GtkBox,
    Button,
    CheckButton,
    ListBox,
    ListBoxRow,
    MessageDialog,
    Orientation,
    ScrolledWindow,
    ButtonsType,
    MessageType,
    Label,
    Separator,
    HeaderBar,
    CssProvider,
    gdk::Display,
    MenuButton,
    gio,
};
use progress::ProgressWindow;
use repo_window::RepoWindow;
use kernel_window::KernelWindow;
use driver_window::DriverWindow;
use std::sync::Mutex;
use std::rc::Rc;
use std::cell::RefCell;


static UPDATE_COUNT: Mutex<i32> = Mutex::new(0);
static CHECKING_UPDATES: Mutex<bool> = Mutex::new(false);

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "
        .header-button {
            padding: 8px 16px;
            margin: 4px;
            border-radius: 8px;
            font-weight: bold;
        }
        
        .refresh-button {
            background: linear-gradient(135deg, #4CAF50, #45a049);
            color: white;
        }
        
        .select-button {
            background: linear-gradient(135deg, #2196F3, #1976D2);
            color: white;
        }
        
        .install-button {
            background: linear-gradient(135deg, #FF9800, #F57C00);
            color: white;
        }
        
        .repo-button {
            background: linear-gradient(135deg, #9C27B0, #7B1FA2);
            color: white;
        }
        
        .package-row {
            padding: 8px;
            border-bottom: 1px solid #e0e0e0;
        }
        
        .package-row:hover {
            background-color: #f5f5f5;
        }
        
        .security-update {
            color: #d32f2f;
            font-weight: bold;
        }
        
        .software-update {
            color: #1976d2;
        }
        
        .kernel-update {
            color: #ff6f00;
            font-weight: bold;
        }
        
        .package-name {
            font-weight: bold;
            font-size: 14px;
        }
        
        .version-info {
            font-family: monospace;
            font-size: 12px;
            color: #666;
        }
        
        .size-info {
            font-size: 12px;
            color: #888;
            font-style: italic;
        }
        
        window {
            background-color: #fafafa;
        }

        .link {
            color: #1976d2;
            text-decoration: underline;
        }
        
        .link:hover {
            color: #1565c0;
            background-color: rgba(25, 118, 210, 0.1);
        }

        .kernel-group {
            margin: 8px;
            padding: 8px;
            border: 1px solid #e0e0e0;
            border-radius: 8px;
            background-color: #fafafa;
        }
        
        .kernel-row {
            padding: 4px;
            border-bottom: 1px solid #f0f0f0;
        }
        
        .kernel-row:hover {
            background-color: #f5f5f5;
        }
        
        .kernel-header-row {
            background-color: #e8e8e8;
            font-weight: bold;
        }
        
        .current-kernel-info {
            background: linear-gradient(135deg, #4CAF50, #45a049);
            color: white;
            padding: 12px;
            border-radius: 8px;
        }

        .kernel-version-list {
            background-color: #fafafa;
            border: 1px solid #e0e0e0;
        }

        .kernel-version-row {
            padding: 8px;
            border-bottom: 1px solid #f0f0f0;
        }

        .kernel-version-row:hover {
            background-color: #e3f2fd;
        }

        .kernel-version-row:selected {
            background-color: #1976d2;
            color: white;
        }

        .kernel-default-btn {
            background: linear-gradient(135deg, #FF9800, #F57C00);
            color: white;
        }

        frame > border {
            border-radius: 8px;
            border: 1px solid #e0e0e0;
        }

        frame > label {
            font-weight: bold;
            color: #1976d2;
        }
        "
    );
    
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn populate_package_list(listbox: &ListBox, packages: Vec<model::PackageUpdate>) {
    populate_package_list_impl(listbox, packages, false);
}

fn populate_package_list_grouped(listbox: &ListBox, packages: Vec<model::PackageUpdate>) {
    populate_package_list_impl(listbox, packages, true);
}

fn populate_package_list_impl(listbox: &ListBox, packages: Vec<model::PackageUpdate>, group_by_type: bool) {

    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }
    

    if let Ok(mut count) = UPDATE_COUNT.lock() {
        *count = packages.len() as i32;
    }
    
    if packages.is_empty() {
        let row = ListBoxRow::new();
        let empty_box = GtkBox::new(Orientation::Horizontal, 12);
        empty_box.set_margin_top(20);
        empty_box.set_margin_bottom(20);
        empty_box.set_halign(gtk::Align::Center);
        
        let empty_label = Label::new(Some("✅ All packages are up to date! No updates required."));
        empty_label.set_markup("<big><b>✅ All packages are up to date!</b></big>");
        empty_box.append(&empty_label);
        
        row.set_child(Some(&empty_box));
        listbox.append(&row);
        return;
    }

    if group_by_type {
        
        let mut kernel_updates = Vec::new();
        let mut security_updates = Vec::new();
        let mut software_updates = Vec::new();
        
        for pkg in packages {
            match pkg.update_type {
                model::UpdateType::Kernel => kernel_updates.push(pkg),
                model::UpdateType::Security => security_updates.push(pkg),
                model::UpdateType::Software => software_updates.push(pkg),
            }
        }
        
        
        kernel_updates.sort_by(|a, b| a.name.cmp(&b.name));
        security_updates.sort_by(|a, b| a.name.cmp(&b.name));
        software_updates.sort_by(|a, b| a.name.cmp(&b.name));
        
        
        if !kernel_updates.is_empty() {
            add_group_header(listbox, "⚡ Kernel Updates", kernel_updates.len());
            for pkg in kernel_updates {
                add_package_row(listbox, pkg);
            }
        }
        
        
        if !security_updates.is_empty() {
            add_group_header(listbox, "🔒 Security Updates", security_updates.len());
            for pkg in security_updates {
                add_package_row(listbox, pkg);
            }
        }
        
        
        if !software_updates.is_empty() {
            add_group_header(listbox, "📦 Software Updates", software_updates.len());
            for pkg in software_updates {
                add_package_row(listbox, pkg);
            }
        }
    } else {
        
        for pkg in packages {
            add_package_row(listbox, pkg);
        }
    }
}

fn add_group_header(listbox: &ListBox, title: &str, count: usize) {
    let header_row = ListBoxRow::new();
    header_row.set_selectable(false);
    header_row.add_css_class("kernel-header-row");
    
    let header_box = GtkBox::new(Orientation::Horizontal, 12);
    header_box.set_margin_top(12);
    header_box.set_margin_bottom(8);
    header_box.set_margin_start(8);
    header_box.set_margin_end(8);
    
    let header_label = Label::new(Some(&format!("{} ({})", title, count)));
    header_label.set_halign(gtk::Align::Start);
    header_label.set_markup(&format!("<b>{} ({})</b>", title, count));
    
    header_box.append(&header_label);
    header_row.set_child(Some(&header_box));
    listbox.append(&header_row);
}

fn add_package_row(listbox: &ListBox, pkg: model::PackageUpdate) {
    let row = ListBoxRow::new();
    row.add_css_class("package-row");
    
    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    
    
    let (type_emoji, type_class) = match pkg.update_type {
        model::UpdateType::Security => ("🔒", "security-update"),
        model::UpdateType::Software => ("📦", "software-update"),
        model::UpdateType::Kernel => ("⚡", "kernel-update"),
    };
    let type_label = Label::new(Some(type_emoji));
    type_label.set_width_chars(6);
    type_label.set_halign(gtk::Align::Center);
    type_label.add_css_class(type_class);
    
    
    let check = CheckButton::new();
    check.set_active(true);
    check.set_halign(gtk::Align::Center);
    
    
    let name_label = Label::new(Some(&pkg.name));
    name_label.set_hexpand(true);
    name_label.set_halign(gtk::Align::Start);
    name_label.add_css_class("package-name");
    
    
    let version_text = if !pkg.current_version.is_empty() {
        format!("{} → {}", pkg.current_version, pkg.new_version)
    } else {
        pkg.new_version.clone()
    };
    let version_label = Label::new(Some(&version_text));
    version_label.set_width_chars(25);
    version_label.set_halign(gtk::Align::Center);
    version_label.add_css_class("version-info");
    
    
    let size_label = Label::new(Some(&pkg.size));
    size_label.set_width_chars(12);
    size_label.set_halign(gtk::Align::Center);
    size_label.add_css_class("size-info");
    
    hbox.append(&type_label);
    hbox.append(&check);
    hbox.append(&name_label);
    hbox.append(&version_label);
    hbox.append(&size_label);
    
    row.set_child(Some(&hbox));
    listbox.append(&row);
}

fn check_updates_background_with_state(listbox: &ListBox, app: &Application, packages_state: &Rc<RefCell<Vec<model::PackageUpdate>>>, _sort_enabled: &Rc<RefCell<bool>>) {
    
    if let Ok(mut checking) = CHECKING_UPDATES.lock() {
        *checking = true;
    }
    
    
    send_notification(app, "checking");
    
    let listbox_clone = listbox.clone();
    let app_clone = app.clone();
    let packages_state_clone = packages_state.clone();
    
    
    glib::spawn_future_local(async move {
        match apt::get_upgradable_packages() {
            Ok(packages) => {
                
                *packages_state_clone.borrow_mut() = packages.clone();
                
                
                populate_package_list(&listbox_clone, packages);
                
                
                if let Ok(mut checking) = CHECKING_UPDATES.lock() {
                    *checking = false;
                }
                
                
                send_notification(&app_clone, "complete");
            }
            Err(e) => {
                
                
                
                if let Ok(mut checking) = CHECKING_UPDATES.lock() {
                    *checking = false;
                }
                
                
                send_notification(&app_clone, "error");
                eprintln!("Update checking error: {}", e);
            }
        }
    });
}

fn send_notification(app: &Application, status: &str) {
    let notification = gio::Notification::new("MeaUpdater");
    
    let count = *UPDATE_COUNT.lock().unwrap_or_else(|e| e.into_inner());
    
    match status {
        "checking" => {
            notification.set_body(Some("Checking for updates..."));
            notification.set_icon(&gio::ThemedIcon::new("software-update-available"));
        }
        "error" => {
            notification.set_body(Some("❌ Checking for updates failed! Please check your internet connection and try again. The system cannot access package repositories."));
            notification.set_icon(&gio::ThemedIcon::new("dialog-error"));
            // Hata bildirimini daha uzun süre göster
            notification.set_priority(gio::NotificationPriority::High);
        }
        "complete" | _ => {
            if count > 0 {
                notification.set_body(Some(&format!("{} updates available", count)));
                notification.set_icon(&gio::ThemedIcon::new("software-update-urgent"));
            } else {
                notification.set_body(Some("✅ Your system is up to date"));
                notification.set_icon(&gio::ThemedIcon::new("software-update-available"));
            }
        }
    }
    
    app.send_notification(Some("update-status"), &notification);
}

fn build_ui(app: &Application) {
    load_css();
    
    let window = ApplicationWindow::builder()
        .application(app)
        .title("MeaUpdater")
        .default_width(850)
        .default_height(625)
        .build();

    
    let header_bar = HeaderBar::new();
    header_bar.set_title_widget(Some(&Label::new(Some("📦 MeaUpdater"))));
    
    
    let menu_model = gio::Menu::new();
    menu_model.append(Some("Sort by Type"), Some("win.sort_by_type"));
    menu_model.append(Some("Kernel Manager"), Some("win.kernels"));
    menu_model.append(Some("Repository Manager"), Some("win.repositories"));
    menu_model.append(Some("Driver Manager"), Some("win.drivers"));
    menu_model.append(Some("About"), Some("win.about"));
    
    
    let menu_button = MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_menu_model(Some(&menu_model));
    
    header_bar.pack_end(&menu_button);
    window.set_titlebar(Some(&header_bar));

    
    let driver_action = gio::ActionEntry::builder("drivers")
        .activate({
            let window = window.clone();
            move |_, _, _| {
                let driver_window = DriverWindow::new(&window);
                driver_window.show();
            }
        })
        .build();

    
    let kernel_action = gio::ActionEntry::builder("kernels")
        .activate({
            let window = window.clone();
            move |_, _, _| {
                let kernel_window = KernelWindow::new(&window);
                kernel_window.show();
            }
        })
        .build();

    
    let repo_action = gio::ActionEntry::builder("repositories")
        .activate({
            let window = window.clone();
            move |_, _, _| {
                let repo_window = RepoWindow::new(&window);
                repo_window.show();
            }
        })
        .build();

    
    let about_action = gio::ActionEntry::builder("about")
        .activate({
            let window = window.clone();
            move |_, _, _| {
                let about_window = about::AboutWindow::new(&window);
                about_window.show();
            }
        })
        .build();
    
    window.add_action_entries([driver_action, kernel_action, repo_action, about_action]);

    let main_vbox = GtkBox::new(Orientation::Vertical, 0);
    
    
    let button_panel = GtkBox::new(Orientation::Horizontal, 12);
    button_panel.set_margin_top(16);
    button_panel.set_margin_bottom(16);
    button_panel.set_margin_start(16);
    button_panel.set_margin_end(16);
    button_panel.set_halign(gtk::Align::Center);

    
    let refresh_btn = Button::with_label("🔄 Check for Updates");
    refresh_btn.add_css_class("header-button");
    refresh_btn.add_css_class("refresh-button");

    let select_all_btn = Button::with_label("☑️ Select/Remove All");
    select_all_btn.add_css_class("header-button");
    select_all_btn.add_css_class("select-button");

    let install_btn = Button::with_label("⬇️ Install Selected");
    install_btn.add_css_class("header-button");
    install_btn.add_css_class("install-button");

    button_panel.append(&refresh_btn);
    button_panel.append(&select_all_btn);
    button_panel.append(&install_btn);
    
    main_vbox.append(&button_panel);
    
    
    let separator = Separator::new(Orientation::Horizontal);
    main_vbox.append(&separator);

    
    let header_row = GtkBox::new(Orientation::Horizontal, 0);
    header_row.set_margin_top(12);
    header_row.set_margin_start(16);
    header_row.set_margin_end(16);
    header_row.set_margin_bottom(8);
    
    let type_header = Label::new(Some("Type"));
    type_header.set_width_chars(6);
    type_header.set_halign(gtk::Align::Center);
    type_header.set_markup("<b>Type</b>");
    
    let select_header = Label::new(Some("Select"));
    select_header.set_width_chars(6);
    select_header.set_halign(gtk::Align::Center);
    select_header.set_markup("<b>Select</b>");
    
    let name_header = Label::new(Some("Package Name"));
    name_header.set_hexpand(true);
    name_header.set_halign(gtk::Align::Start);
    name_header.set_markup("<b>Package Name</b>");
    
    let version_header = Label::new(Some("Version"));
    version_header.set_width_chars(25);
    version_header.set_halign(gtk::Align::Center);
    version_header.set_markup("<b>Version</b>");
    
    let size_header = Label::new(Some("Size"));
    size_header.set_width_chars(12);
    size_header.set_halign(gtk::Align::Center);
    size_header.set_markup("<b>Size</b>");
    
    header_row.append(&type_header);
    header_row.append(&select_header);
    header_row.append(&name_header);
    header_row.append(&version_header);
    header_row.append(&size_header);
    
    main_vbox.append(&header_row);
    
    
    let separator2 = Separator::new(Orientation::Horizontal);
    main_vbox.append(&separator2);

    
    let scrolled_window = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .margin_top(8)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
        
    let listbox = ListBox::new();
    listbox.set_selection_mode(gtk::SelectionMode::None);
    scrolled_window.set_child(Some(&listbox));
    main_vbox.append(&scrolled_window);

    window.set_child(Some(&main_vbox));

    
    let current_packages = Rc::new(RefCell::new(Vec::<model::PackageUpdate>::new()));

    
    let sort_action = gio::SimpleAction::new_stateful(
        "sort_by_type",
        None,
        &false.to_variant(),
    );
    
    let listbox_sort = listbox.clone();
    let packages_sort = current_packages.clone();
    sort_action.connect_activate({
        move |action, _| {
            let current_state = action.state().unwrap().get::<bool>().unwrap();
            let new_state = !current_state;
            action.set_state(&new_state.to_variant());
            
            if new_state {
                populate_package_list_grouped(&listbox_sort, packages_sort.borrow().clone());
            } else {
                populate_package_list(&listbox_sort, packages_sort.borrow().clone());
            }
        }
    });
    
    window.add_action(&sort_action);

    
    let app_clone_for_close = app.clone();
    window.connect_close_request(move |_| {
        
        app_clone_for_close.quit();
        glib::Propagation::Proceed
    });
    
      
    check_updates_background_with_state(&listbox, app, &current_packages, &Rc::new(RefCell::new(false)));
    
    window.present();

    let window1 = window.clone();
    let window2 = window.clone();

    
    let select_all_list = listbox.clone();
    select_all_btn.connect_clicked(move |_| {
        let mut child = select_all_list.first_child();
        let mut all_selected = true;
        
        while let Some(row_widget) = child {
            child = row_widget.next_sibling();
            if let Some(hbox) = row_widget
                .downcast::<ListBoxRow>().ok()
                .and_then(|r| r.child())
                .and_then(|c| c.downcast::<GtkBox>().ok())
            {
                let mut btn_child = hbox.first_child();
                while let Some(widget) = btn_child {
                    btn_child = widget.next_sibling();
                    if let Ok(check) = widget.downcast::<CheckButton>() {
                        if !check.is_active() {
                            all_selected = false;
                            break;
                        }
                    }
                }
            }
            if !all_selected { break; }
        }
        
        let mut child2 = select_all_list.first_child();
        while let Some(row_widget) = child2 {
            child2 = row_widget.next_sibling();
            if let Some(hbox) = row_widget
                .downcast::<ListBoxRow>().ok()
                .and_then(|r| r.child())
                .and_then(|c| c.downcast::<GtkBox>().ok())
            {
                let mut btn_child = hbox.first_child();
                while let Some(widget) = btn_child {
                    btn_child = widget.next_sibling();
                    if let Ok(check) = widget.downcast::<CheckButton>() {
                        check.set_active(!all_selected);
                    }
                }
            }
        }
    });

    
    let listbox_clone = listbox.clone();
    let refresh_window = window1.clone();
    let app_clone2 = app.clone();
    let current_packages_refresh = current_packages.clone();
    refresh_btn.connect_clicked(move |_| {
        
        let progress_window = ProgressWindow::new(&refresh_window);
        progress_window.show();
        
        
        if let Ok(mut checking) = CHECKING_UPDATES.lock() {
            *checking = true;
        }
        
        
        send_notification(&app_clone2, "checking");
        
        
        let listbox_for_update = listbox_clone.clone();
        let progress_window_clone = progress_window.clone();
        let app_clone3 = app_clone2.clone();
        let refresh_window_clone = refresh_window.clone();
        let current_packages_async = current_packages_refresh.clone();
        
        glib::spawn_future_local(async move {
            match progress_window_clone.check_updates_with_progress().await {
                Ok(packages) => {
                    
                    *current_packages_async.borrow_mut() = packages.clone();
                    
                    
                    if let Some(action) = refresh_window_clone.lookup_action("sort_by_type") {
                        if let Some(simple_action) = action.downcast_ref::<gio::SimpleAction>() {
                            let is_grouped = simple_action.state().unwrap().get::<bool>().unwrap();
                            if is_grouped {
                                populate_package_list_grouped(&listbox_for_update, packages);
                            } else {
                                populate_package_list(&listbox_for_update, packages);
                            }
                        } else {
                            populate_package_list(&listbox_for_update, packages);
                        }
                    } else {
                        populate_package_list(&listbox_for_update, packages);
                    }
                    
                    
                    if let Ok(mut checking) = CHECKING_UPDATES.lock() {
                        *checking = false;
                    }
                    
                    
                    send_notification(&app_clone3, "complete");
                }
                Err(e) => {
                
                    
                    
                    if let Ok(mut checking) = CHECKING_UPDATES.lock() {
                        *checking = false;
                    }
                    
                    
                    send_notification(&app_clone3, "error");
                    
                    
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&refresh_window_clone)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text("❌ Update Check Failed!")
                        .secondary_text(&format!(
                            "An error occurred while checking for updates:\n\n{}\n\n\
                            Please check your internet connection and try again.",
                            e
                        ))
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                }
            }
        });
    });

    
    let install_window = window2;
    let listbox_for_install = listbox.clone();
    let refresh_clone2 = refresh_btn.clone();
    install_btn.connect_clicked(move |_| {
        let mut selected = Vec::new();
        let mut child = listbox_for_install.first_child();
        while let Some(row_widget) = child {
            child = row_widget.next_sibling();
            if let Some(hbox) = row_widget
                .downcast::<ListBoxRow>().ok()
                .and_then(|r| r.child())
                .and_then(|c| c.downcast::<GtkBox>().ok())
            {
                let mut checkbox_found = false;
                let mut package_name = String::new();
                
                let mut widget_child = hbox.first_child();
                let mut widget_count = 0;
                
                while let Some(widget) = widget_child {
                    widget_child = widget.next_sibling();
                    widget_count += 1;
                    
                    
                    if let Ok(check) = widget.clone().downcast::<CheckButton>() {
                        checkbox_found = check.is_active();
                    }
                    
                    
                    if widget_count == 3 {
                        if let Ok(label) = widget.clone().downcast::<Label>() {
                            package_name = label.text().to_string();
                        }
                    }
                }
                
                if checkbox_found && !package_name.is_empty() {
                    selected.push(package_name);
                }
            }
        }

        if selected.is_empty() {
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Error)
                .buttons(ButtonsType::Ok)
                .text("⚠️ Please select at least one package.")
                .build();
            dialog.connect_response(|dlg, _| dlg.close());
            dialog.show();
            return;
        }

        
        let progress_window = ProgressWindow::new(&install_window);
        progress_window.show();
        
        
        let install_window_clone = install_window.clone();
        let refresh_clone2_clone = refresh_clone2.clone();
        
        
        let progress_window_clone = progress_window.clone();
        glib::spawn_future_local(async move {
            if let Err(err) = progress_window_clone.install_packages_with_progress(&selected).await {
                let dialog = MessageDialog::builder()
                    .transient_for(&install_window_clone)
                    .modal(true)
                    .message_type(MessageType::Error)
                    .buttons(ButtonsType::Ok)
                    .text(&format!("❌ Installation initialization error:\n{}", err))
                    .build();
                dialog.connect_response(|dlg, _| dlg.close());
                dialog.show();
            } else {
                
                glib::timeout_add_seconds_local(3, {
                    let refresh_btn = refresh_clone2_clone.clone();
                    move || {
                        refresh_btn.emit_clicked();
                        glib::ControlFlow::Break
                    }
                });
            }
        });
    });
    
    
    let listbox_periodic = listbox.clone();
    let app_periodic = app.clone();
    let packages_periodic = current_packages.clone();
    let sort_periodic = Rc::new(RefCell::new(false));
    glib::timeout_add_seconds_local(1800, move || { 
        check_updates_background_with_state(&listbox_periodic, &app_periodic, &packages_periodic, &sort_periodic);
        glib::ControlFlow::Continue
    });
    
    
    let window_for_activation = window.clone();
    app.connect_activate(move |_| {
        window_for_activation.set_visible(true);
        window_for_activation.present();
    });
}

fn main() -> Result<(), Error> {
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    
    let app = Application::builder()
        .application_id("org.mthakan.meaupdater")
        .build();

    app.connect_activate(build_ui);
    app.run();
    Ok(())
}
