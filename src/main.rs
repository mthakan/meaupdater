mod apt;
mod model;
mod policy;

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
    ProgressBar,
    ScrolledWindow,
    ButtonsType,
    MessageType,
    Label,
    Separator,
    HeaderBar,
    CssProvider,
    gdk::Display,
};
use std::process::Command;

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
        "
    );
    
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application) {
    load_css();
    
    let window = ApplicationWindow::builder()
        .application(app)
        .title("MeaUpdater")
        .default_width(800)
        .default_height(600)
        .build();

    // Adding header bar
    let header_bar = HeaderBar::new();
    header_bar.set_title_widget(Some(&Label::new(Some("📦 MeaUpdater"))));
    window.set_titlebar(Some(&header_bar));

    let main_vbox = GtkBox::new(Orientation::Vertical, 0);
    
    // Top panel for buttons
    let button_panel = GtkBox::new(Orientation::Horizontal, 12);
    button_panel.set_margin_top(16);
    button_panel.set_margin_bottom(16);
    button_panel.set_margin_start(16);
    button_panel.set_margin_end(16);
    button_panel.set_halign(gtk::Align::Center);

    // Modern buttons
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
    
    // Separating line
    let separator = Separator::new(Orientation::Horizontal);
    main_vbox.append(&separator);

    // List header row
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
    
    // Bottom separator
    let separator2 = Separator::new(Orientation::Horizontal);
    main_vbox.append(&separator2);

    // Main list
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
    window.present();

    let window1 = window.clone();
    let window2 = window.clone();

    
    let select_all_list = listbox.clone();
    select_all_btn.connect_clicked(move |_| {
        let mut child = select_all_list.first_child();
        let mut all_selected = true;
        // first check if all are selected
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
        // select or remove depending on the situation
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
    let refresh_clone = refresh_btn.clone();
    let refresh_window = window1.clone();
    refresh_btn.connect_clicked(move |_| {
        while let Some(child) = listbox_clone.first_child() {
            listbox_clone.remove(&child);
        }
        match apt::get_upgradable_packages() {
            Ok(pkgs) if pkgs.is_empty() => {
                let row = ListBoxRow::new();
                let empty_box = GtkBox::new(Orientation::Horizontal, 12);
                empty_box.set_margin_top(20);
                empty_box.set_margin_bottom(20);
                empty_box.set_halign(gtk::Align::Center);
                
                let empty_label = Label::new(Some("✅ All packages are up to date! No updates required."));
                empty_label.set_markup("<big><b>✅ All packages are up to date!</b></big>");
                empty_box.append(&empty_label);
                
                row.set_child(Some(&empty_box));
                listbox_clone.append(&row);
            }
            Ok(pkgs) => {
                for pkg in pkgs {
                    let row = ListBoxRow::new();
                    row.add_css_class("package-row");
                    
                    let hbox = GtkBox::new(Orientation::Horizontal, 12);
                    hbox.set_margin_top(8);
                    hbox.set_margin_bottom(8);
                    hbox.set_margin_start(8);
                    hbox.set_margin_end(8);
                    
                    // 1. Type emoji
                    let type_emoji = match pkg.update_type {
                        model::UpdateType::Security => "🔒",
                        model::UpdateType::Software => "📦",
                    };
                    let type_label = Label::new(Some(type_emoji));
                    type_label.set_width_chars(6);
                    type_label.set_halign(gtk::Align::Center);
                    if pkg.update_type == model::UpdateType::Security {
                        type_label.add_css_class("security-update");
                    } else {
                        type_label.add_css_class("software-update");
                    }
                    
                    // 2. Checkbox
                    let check = CheckButton::new();
                    check.set_active(true);
                    check.set_halign(gtk::Align::Center);
                    
                    // 3. Package name
                    let name_label = Label::new(Some(&pkg.name));
                    name_label.set_hexpand(true);
                    name_label.set_halign(gtk::Align::Start);
                    name_label.add_css_class("package-name");
                    
                    // 4. Version information
                    let version_text = if !pkg.current_version.is_empty() {
                        format!("{} → {}", pkg.current_version, pkg.new_version)
                    } else {
                        pkg.new_version.clone()
                    };
                    let version_label = Label::new(Some(&version_text));
                    version_label.set_width_chars(25);
                    version_label.set_halign(gtk::Align::Center);
                    version_label.add_css_class("version-info");
                    
                    // 5. Size information - a simple approach, size calculation temporarily removed
                    let size_label = Label::new(Some("~MB"));
                    size_label.set_width_chars(12);
                    size_label.set_halign(gtk::Align::Center);
                    size_label.add_css_class("size-info");
                    
                    hbox.append(&type_label);
                    hbox.append(&check);
                    hbox.append(&name_label);
                    hbox.append(&version_label);
                    hbox.append(&size_label);
                    
                    row.set_child(Some(&hbox));
                    listbox_clone.append(&row);
                }
            }
            Err(err) => {
                let dialog = MessageDialog::builder()
                    .transient_for(&refresh_window)
                    .modal(true)
                    .message_type(MessageType::Error)
                    .buttons(ButtonsType::Ok)
                    .text(&format!("Error while getting update:\n{}", err))
                    .build();
                dialog.connect_response(|dlg, _| dlg.close());
                dialog.show();
            }
        }
    });

    // Install proccess
    let install_window = window2;
    let listbox_for_install = listbox.clone();
    let refresh_clone2 = refresh_clone;
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
                let mut is_third_widget = false; // To follow the 3rd Widget
                let mut widget_count = 0;
                
                while let Some(widget) = widget_child {
                    widget_child = widget.next_sibling();
                    widget_count += 1;
                    
                    // Find CheckButton 
                    if let Ok(check) = widget.clone().downcast::<CheckButton>() {
                        checkbox_found = check.is_active();
                    }
                    
                    // Find package name 
                    if widget_count == 3 { // 3. widget
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

        let progress = ProgressBar::new();
        progress.set_show_text(true);
        let pg_dialog = MessageDialog::builder()
            .transient_for(&install_window)
            .modal(true)
            .message_type(MessageType::Other)
            .buttons(ButtonsType::None)
            .text("⬇️ Installing updates…")
            .build();
        pg_dialog.content_area().append(&progress);
        pg_dialog.show();

        if let Err(err) = policy::install_packages(&selected) {
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Error)
                .buttons(ButtonsType::Ok)
                .text(&format!("❌ Installation error:\n{}", err))
                .build();
            dialog.connect_response(|dlg, _| dlg.close());
            dialog.show();
        } else {
            progress.set_fraction(1.0);
            progress.set_text(Some("✅ Completed"));
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Info)
                .buttons(ButtonsType::Ok)
                .text("✅ Updates installed successfully.")
                .build();
            dialog.connect_response(|dlg, _| dlg.close());
            dialog.show();
            refresh_clone2.emit_clicked();
        }
        pg_dialog.close();
    });
}

fn main() -> Result<(), Error> {
    let app = Application::builder()
        .application_id("org.mthakan.meaupdater")
        .build();

    app.connect_activate(build_ui);
    app.run();
    Ok(())
}
