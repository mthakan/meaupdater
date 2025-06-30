// src/repo_window.rs

use crate::repo_manager::{self, Repository};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, ListBox, ListBoxRow, ScrolledWindow,
    Orientation, HeaderBar, Label, Entry, Dialog, MessageDialog,
    ButtonsType, MessageType, CheckButton, Separator, Grid,
    ResponseType,
};
use std::rc::Rc;
use std::cell::RefCell;

pub struct RepoWindow {
    window: Dialog,
}

impl RepoWindow {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let window = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Repository Manager")
            .default_width(1000)
            .default_height(600)
            .build();

        // Header bar
        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("🗂️ APT Repository Manager"))));
        window.set_titlebar(Some(&header_bar));

        let main_vbox = GtkBox::new(Orientation::Vertical, 0);

        // Button panel
        let button_panel = GtkBox::new(Orientation::Horizontal, 12);
        button_panel.set_margin_top(16);
        button_panel.set_margin_bottom(16);
        button_panel.set_margin_start(16);
        button_panel.set_margin_end(16);
        button_panel.set_halign(gtk::Align::Center);

        let refresh_btn = Button::with_label("🔄 Refresh");
        refresh_btn.add_css_class("header-button");
        refresh_btn.add_css_class("refresh-button");

        let add_btn = Button::with_label("➕ Add Repository");
        add_btn.add_css_class("header-button");
        add_btn.add_css_class("select-button");

        let update_btn = Button::with_label("⬇️ Update Repositories");
        update_btn.add_css_class("header-button");
        update_btn.add_css_class("install-button");

        button_panel.append(&refresh_btn);
        button_panel.append(&add_btn);
        button_panel.append(&update_btn);

        main_vbox.append(&button_panel);

        
        let separator = Separator::new(Orientation::Horizontal);
        main_vbox.append(&separator);

        // List title
        let header_row = GtkBox::new(Orientation::Horizontal, 0);
        header_row.set_margin_top(12);
        header_row.set_margin_start(16);
        header_row.set_margin_end(16);
        header_row.set_margin_bottom(8);

        let status_header = Label::new(Some("Status"));
        status_header.set_width_chars(8);
        status_header.set_halign(gtk::Align::Center);
        status_header.set_markup("<b>Status</b>");

        let name_header = Label::new(Some("Repository Name"));
        name_header.set_width_chars(25);
        name_header.set_halign(gtk::Align::Start);
        name_header.set_markup("<b>Repository Name</b>");

        let uri_header = Label::new(Some("URI"));
        uri_header.set_hexpand(true);
        uri_header.set_halign(gtk::Align::Start);
        uri_header.set_markup("<b>URI</b>");

        let dist_header = Label::new(Some("Distribution"));
        dist_header.set_width_chars(15);
        dist_header.set_halign(gtk::Align::Center);
        dist_header.set_markup("<b>Distribution</b>");

        let comp_header = Label::new(Some("Components"));
        comp_header.set_width_chars(20);
        comp_header.set_halign(gtk::Align::Center);
        comp_header.set_markup("<b>Components</b>");

        let actions_header = Label::new(Some("Actions"));
        actions_header.set_width_chars(15);
        actions_header.set_halign(gtk::Align::Center);
        actions_header.set_markup("<b>Actions</b>");

        header_row.append(&status_header);
        header_row.append(&name_header);
        header_row.append(&uri_header);
        header_row.append(&dist_header);
        header_row.append(&comp_header);
        header_row.append(&actions_header);

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

        let content_area = window.content_area();
        content_area.append(&main_vbox);

        let repositories = Rc::new(RefCell::new(Vec::new()));

        // Event handlers
        let repos_clone = repositories.clone();
        let listbox_clone = listbox.clone();
        refresh_btn.connect_clicked(move |_| {
            Self::refresh_repositories(&repos_clone, &listbox_clone);
        });

        let window_clone = window.clone();
        let repos_clone2 = repositories.clone();
        let listbox_clone2 = listbox.clone();
        add_btn.connect_clicked(move |_| {
            Self::show_add_dialog(&window_clone, &repos_clone2, &listbox_clone2);
        });

        let window_clone2 = window.clone();
        update_btn.connect_clicked(move |_| {
            Self::update_repositories(&window_clone2);
        });

        // First load
        Self::refresh_repositories(&repositories, &listbox);

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn refresh_repositories(repositories: &Rc<RefCell<Vec<Repository>>>, listbox: &ListBox) {
        // Clear list
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        // Read the real data again
        match repo_manager::get_repositories() {
            Ok(repos) => {
                *repositories.borrow_mut() = repos.clone();
                Self::populate_repository_list(listbox, repos, repositories);
            }
            Err(e) => {
                let error_row = ListBoxRow::new();
                let error_label = Label::new(Some(&format!("❌ Error: {}", e)));
                error_label.set_margin_top(20);
                error_label.set_margin_bottom(20);
                error_row.set_child(Some(&error_label));
                listbox.append(&error_row);
            }
        }
    }

    fn populate_repository_list(listbox: &ListBox, repositories: Vec<Repository>, repositories_ref: &Rc<RefCell<Vec<Repository>>>) {
        if repositories.is_empty() {
            let row = ListBoxRow::new();
            let empty_label = Label::new(Some("📂 Repository not found"));
            empty_label.set_margin_top(20);
            empty_label.set_margin_bottom(20);
            empty_label.set_halign(gtk::Align::Center);
            row.set_child(Some(&empty_label));
            listbox.append(&row);
            return;
        }

        for (index, repo) in repositories.iter().enumerate() {
            let row = ListBoxRow::new();
            row.add_css_class("package-row");

            let hbox = GtkBox::new(Orientation::Horizontal, 12);
            hbox.set_margin_top(8);
            hbox.set_margin_bottom(8);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);

            // Status checkbox
            let status_check = CheckButton::new();
            status_check.set_active(repo.enabled);
            status_check.set_width_request(60);
            status_check.set_halign(gtk::Align::Center);

            // Checkbox event handler - Change the repo status
            let repo_clone = repo.clone();
            let repos_ref_clone = repositories_ref.clone();
            let listbox_clone = listbox.clone();
            status_check.connect_toggled(move |check| {
                let is_active = check.is_active();
                if is_active != repo_clone.enabled {
                    match repo_manager::toggle_repository(&repo_clone) {
                        Ok(_) => {
                            // Refresh list
                            Self::refresh_repositories(&repos_ref_clone, &listbox_clone);
                        }
                        Err(e) => {
                            eprintln!("The repository status could not be changed: {}", e);
                            // Revert checkbox to its previous state
                            check.set_active(repo_clone.enabled);
                        }
                    }
                }
            });

            // Repository name
            let name_label = Label::new(Some(&repo.name));
            name_label.set_width_chars(25);
            name_label.set_halign(gtk::Align::Start);
            name_label.add_css_class("package-name");
            if repo.is_source {
                name_label.set_markup(&format!("<i>{} (source)</i>", repo.name));
            }

            // URI
            let uri_label = Label::new(Some(&repo.uri));
            uri_label.set_hexpand(true);
            uri_label.set_halign(gtk::Align::Start);
            uri_label.set_ellipsize(pango::EllipsizeMode::Middle);
            uri_label.add_css_class("version-info");

            // Distro
            let dist_label = Label::new(Some(&repo.distribution));
            dist_label.set_width_chars(15);
            dist_label.set_halign(gtk::Align::Center);
            dist_label.add_css_class("version-info");

            // Components
            let comp_label = Label::new(Some(&repo.components));
            comp_label.set_width_chars(20);
            comp_label.set_halign(gtk::Align::Center);
            comp_label.set_ellipsize(pango::EllipsizeMode::End);
            comp_label.add_css_class("size-info");

            // Action buttons
            let actions_box = GtkBox::new(Orientation::Horizontal, 4);
            actions_box.set_width_request(120);
            actions_box.set_halign(gtk::Align::Center);

            let edit_btn = Button::from_icon_name("document-edit-symbolic");
            edit_btn.set_tooltip_text(Some("Düzenle"));
            edit_btn.add_css_class("flat");

            let delete_btn = Button::from_icon_name("user-trash-symbolic");
            delete_btn.set_tooltip_text(Some("Sil"));
            delete_btn.add_css_class("flat");
            delete_btn.add_css_class("destructive-action");

            // Edit button event handler
            let repo_edit_clone = repo.clone();
            let repos_ref_edit_clone = repositories_ref.clone();
            let listbox_edit_clone = listbox.clone();
            edit_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                    Self::show_edit_dialog(&window, &repo_edit_clone, &repos_ref_edit_clone, &listbox_edit_clone);
                }
            });

            // Delete button event handler
            let repo_delete_clone = repo.clone();
            let repos_ref_delete_clone = repositories_ref.clone();
            let listbox_delete_clone = listbox.clone();
            delete_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                    Self::show_delete_confirmation(&window, &repo_delete_clone, &repos_ref_delete_clone, &listbox_delete_clone);
                }
            });

            actions_box.append(&edit_btn);
            actions_box.append(&delete_btn);

            hbox.append(&status_check);
            hbox.append(&name_label);
            hbox.append(&uri_label);
            hbox.append(&dist_label);
            hbox.append(&comp_label);
            hbox.append(&actions_box);

            row.set_child(Some(&hbox));
            listbox.append(&row);
        }
    }

    fn show_add_dialog(parent: &Dialog, repositories: &Rc<RefCell<Vec<Repository>>>, listbox: &ListBox) {
        let dialog = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title("Add New Repository")
            .default_width(500)
            .default_height(300)
            .build();

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Add", ResponseType::Accept);

        let content_area = dialog.content_area();
        let grid = Grid::new();
        grid.set_margin_top(20);
        grid.set_margin_bottom(20);
        grid.set_margin_start(20);
        grid.set_margin_end(20);
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);

        // URI
        let uri_label = Label::new(Some("Repository URI:"));
        uri_label.set_halign(gtk::Align::Start);
        let uri_entry = Entry::new();
        uri_entry.set_placeholder_text(Some("http://deb.debian.org/debian"));
        uri_entry.set_hexpand(true);

        // Distro
        let dist_label = Label::new(Some("Distro:"));
        dist_label.set_halign(gtk::Align::Start);
        let dist_entry = Entry::new();
        dist_entry.set_placeholder_text(Some("bullseye"));

        // Components
        let comp_label = Label::new(Some("Components:"));
        comp_label.set_halign(gtk::Align::Start);
        let comp_entry = Entry::new();
        comp_entry.set_placeholder_text(Some("main contrib non-free"));

        grid.attach(&uri_label, 0, 0, 1, 1);
        grid.attach(&uri_entry, 1, 0, 1, 1);
        grid.attach(&dist_label, 0, 1, 1, 1);
        grid.attach(&dist_entry, 1, 1, 1, 1);
        grid.attach(&comp_label, 0, 2, 1, 1);
        grid.attach(&comp_entry, 1, 2, 1, 1);

        content_area.append(&grid);

        let repos_clone = repositories.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                let uri = uri_entry.text().to_string();
                let dist = dist_entry.text().to_string();
                let comp = comp_entry.text().to_string();

                if uri.is_empty() || dist.is_empty() || comp.is_empty() {
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text("❌ Please fill in all fields!")
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                    return;
                }

                match repo_manager::add_repository(&uri, &dist, &comp) {
                    Ok(_) => {
                        Self::refresh_repositories(&repos_clone, &listbox_clone);
                        dialog.close();
                        
                        let success_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Repository added successfully!")
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
                            .text(&format!("❌ The repository could not be added:\n{}", e))
                            .build();
                        error_dialog.connect_response(|dlg, _| dlg.close());
                        error_dialog.show();
                    }
                }
            } else {
                dialog.close();
            }
        });

        dialog.show();
    }

    fn show_edit_dialog(parent: &gtk::Window, repo: &Repository, repositories: &Rc<RefCell<Vec<Repository>>>, listbox: &ListBox) {
        let dialog = Dialog::builder()
            .transient_for(parent)
            .modal(true)
            .title(&format!("Edit Repository: {}", repo.name))
            .default_width(500)
            .default_height(300)
            .build();

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Save", ResponseType::Accept);

        let content_area = dialog.content_area();
        let grid = Grid::new();
        grid.set_margin_top(20);
        grid.set_margin_bottom(20);
        grid.set_margin_start(20);
        grid.set_margin_end(20);
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);

        // URI
        let uri_label = Label::new(Some("Repository URI:"));
        uri_label.set_halign(gtk::Align::Start);
        let uri_entry = Entry::new();
        uri_entry.set_text(&repo.uri);
        uri_entry.set_hexpand(true);

        // Distro
        let dist_label = Label::new(Some("Distro:"));
        dist_label.set_halign(gtk::Align::Start);
        let dist_entry = Entry::new();
        dist_entry.set_text(&repo.distribution);

        // Components
        let comp_label = Label::new(Some("Components:"));
        comp_label.set_halign(gtk::Align::Start);
        let comp_entry = Entry::new();
        comp_entry.set_text(&repo.components);

        grid.attach(&uri_label, 0, 0, 1, 1);
        grid.attach(&uri_entry, 1, 0, 1, 1);
        grid.attach(&dist_label, 0, 1, 1, 1);
        grid.attach(&dist_entry, 1, 1, 1, 1);
        grid.attach(&comp_label, 0, 2, 1, 1);
        grid.attach(&comp_entry, 1, 2, 1, 1);

        content_area.append(&grid);

        let repo_clone = repo.clone();
        let repos_clone = repositories.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                let uri = uri_entry.text().to_string();
                let dist = dist_entry.text().to_string();
                let comp = comp_entry.text().to_string();

                if uri.is_empty() || dist.is_empty() || comp.is_empty() {
                    let error_dialog = MessageDialog::builder()
                        .transient_for(&parent_clone)
                        .modal(true)
                        .message_type(MessageType::Error)
                        .buttons(ButtonsType::Ok)
                        .text("❌ Please fill in all fields!")
                        .build();
                    error_dialog.connect_response(|dlg, _| dlg.close());
                    error_dialog.show();
                    return;
                }

                match repo_manager::edit_repository(&repo_clone, &uri, &dist, &comp) {
                    Ok(_) => {
                        Self::refresh_repositories(&repos_clone, &listbox_clone);
                        dialog.close();
                        
                        let success_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Repository successfully edited!")
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
                            .text(&format!("❌ Repository could not be edited:\n{}", e))
                            .build();
                        error_dialog.connect_response(|dlg, _| dlg.close());
                        error_dialog.show();
                    }
                }
            } else {
                dialog.close();
            }
        });

        dialog.show();
    }

    fn show_delete_confirmation(parent: &gtk::Window, repo: &Repository, repositories: &Rc<RefCell<Vec<Repository>>>, listbox: &ListBox) {
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::YesNo)
            .text(&format!("🗑️ Repository Deletion Confirmation"))
            .secondary_text(&format!("Are you sure you want to delete the '{}' repository?\n\nURI: {}", repo.name, repo.uri))
            .build();

        let repo_clone = repo.clone();
        let repos_clone = repositories.clone();
        let listbox_clone = listbox.clone();
        let parent_clone = parent.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Yes {
                match repo_manager::remove_repository(&repo_clone) {
                    Ok(_) => {
                        Self::refresh_repositories(&repos_clone, &listbox_clone);
                        
                        let success_dialog = MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .message_type(MessageType::Info)
                            .buttons(ButtonsType::Ok)
                            .text("✅ Repository deleted successfully!")
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
                            .text(&format!("❌ Repository could not be deleted:\n{}", e))
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

    fn update_repositories(parent: &Dialog) {
        match repo_manager::update_repositories() {
            Ok(_) => {
                let success_dialog = MessageDialog::builder()
                    .transient_for(parent)
                    .modal(true)
                    .message_type(MessageType::Info)
                    .buttons(ButtonsType::Ok)
                    .text("✅ Repository list updated successfully!")
                    .build();
                success_dialog.connect_response(|dlg, _| dlg.close());
                success_dialog.show();
            }
            Err(e) => {
                let error_dialog = MessageDialog::builder()
                    .transient_for(parent)
                    .modal(true)
                    .message_type(MessageType::Error)
                    .buttons(ButtonsType::Ok)
                    .text(&format!("❌ Repository update failed:\n{}", e))
                    .build();
                error_dialog.connect_response(|dlg, _| dlg.close());
                error_dialog.show();
            }
        }
    }
}
