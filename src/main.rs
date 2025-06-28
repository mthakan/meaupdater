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
};

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("MeaUpdater")
        .default_width(600)
        .default_height(400)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    let scrolled_window = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    let listbox = ListBox::new();
    scrolled_window.set_child(Some(&listbox));
    vbox.append(&scrolled_window);

    let refresh_btn = Button::with_label("Refresh");
    vbox.append(&refresh_btn);

    // "Select All" button
    let select_all_btn = Button::with_label("Select all");
    vbox.append(&select_all_btn);

    let install_btn = Button::with_label("Install Selected Updates");
    vbox.append(&install_btn);

    window.set_child(Some(&vbox));
    window.present();

    let window1 = window.clone();
    let window2 = window.clone();

    // Refresh process
    let listbox_clone = listbox.clone();
    let refresh_clone = refresh_btn.clone();
    let refresh_window = window1.clone();
    let select_all_clone = select_all_btn.clone();
    // Select all
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

    refresh_btn.connect_clicked(move |_| {
        while let Some(child) = listbox_clone.first_child() {
            listbox_clone.remove(&child);
        }
        match apt::get_upgradable_packages() {
            Ok(pkgs) if pkgs.is_empty() => {
                let row = ListBoxRow::new();
                let hbox = GtkBox::new(Orientation::Horizontal, 10);
                let check = CheckButton::with_label("No updatable packages.");
                check.set_active(false);
                hbox.append(&check);
                row.set_child(Some(&hbox));
                listbox_clone.append(&row);
            }
            Ok(pkgs) => {
                for pkg in pkgs {
                    let row = ListBoxRow::new();
                    let hbox = GtkBox::new(Orientation::Horizontal, 10);
                    let check = CheckButton::new();
                    check.set_label(Some(&format!(
                        "{}: {} → {} ({:?})",
                        pkg.name, pkg.current_version, pkg.new_version, pkg.update_type
                    )));
                    check.set_active(true);
                    hbox.append(&check);
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

    // Installation process
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
                let mut btn_child = hbox.first_child();
                while let Some(widget) = btn_child {
                    btn_child = widget.next_sibling();
                    if let Ok(check) = widget.downcast::<CheckButton>() {
                        if check.is_active() {
                            if let Some(label) = check.label() {
                                if let Some(name) = label.split(':').next() {
                                    selected.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        if selected.is_empty() {
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Error)
                .buttons(ButtonsType::Ok)
                .text("Please select at least one package.")
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
            .text("Installing updates…")
            .build();
        pg_dialog.content_area().append(&progress);
        pg_dialog.show();

        if let Err(err) = policy::install_packages(&selected) {
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Error)
                .buttons(ButtonsType::Ok)
                .text(&format!("Installation error:\n{}", err))
                .build();
            dialog.connect_response(|dlg, _| dlg.close());
            dialog.show();
        } else {
            progress.set_fraction(1.0);
            progress.set_text(Some("Completed"));
            let dialog = MessageDialog::builder()
                .transient_for(&install_window)
                .modal(true)
                .message_type(MessageType::Info)
                .buttons(ButtonsType::Ok)
                .text("Updates installed successfully.")
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
