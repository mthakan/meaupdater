// src/about.rs

use gtk::prelude::*;
use gtk::{
    Window, ApplicationWindow, Box as GtkBox, Label, Button,
    Orientation, gdk_pixbuf::Pixbuf, Picture, HeaderBar,
};

pub struct AboutWindow;

impl AboutWindow {
    pub fn new(parent: &ApplicationWindow) -> Window {
        let window = Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("About MeaUpdater")
            .default_width(400)
            .default_height(350)
            .resizable(false)
            .build();

        // Header bar
        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("About"))));
        window.set_titlebar(Some(&header_bar));

        // Main container 
        let main_vbox = GtkBox::new(Orientation::Vertical, 0);
        main_vbox.set_halign(gtk::Align::Center);
        main_vbox.set_valign(gtk::Align::Center);
        main_vbox.set_margin_top(30);
        main_vbox.set_margin_bottom(30);
        main_vbox.set_margin_start(40);
        main_vbox.set_margin_end(40);

        // Logo area
        let logo_container = GtkBox::new(Orientation::Vertical, 0);
        logo_container.set_halign(gtk::Align::Center);
        logo_container.set_margin_bottom(25);

        // Logo
        let logo_data = include_bytes!("../assets/logo.png");
        match Pixbuf::from_read(std::io::Cursor::new(logo_data)) {
            Ok(pixbuf) => {
                // Resize logo proportionally (maximum 160x160)
                let scaled_pixbuf = if pixbuf.width() > 160 || pixbuf.height() > 160 {
                    pixbuf.scale_simple(160, 160, gdk_pixbuf::InterpType::Bilinear)
                        .unwrap_or(pixbuf)
                } else {
                    pixbuf
                };
                
                let picture = Picture::for_pixbuf(&scaled_pixbuf);
                picture.set_halign(gtk::Align::Center);
                picture.set_size_request(160, 160);
                picture.set_can_shrink(false);
                logo_container.append(&picture);
            }
            Err(_) => {
                // If logo cannot be loaded, use emoji
                let logo_label = Label::new(Some("📦"));
                logo_label.set_markup("<span size='xx-large'>📦</span>");
                logo_label.set_halign(gtk::Align::Center);
                logo_container.append(&logo_label);
            }
        }

        main_vbox.append(&logo_container);

        // title
        let title_label = Label::new(Some("MeaUpdater"));
        title_label.set_markup("<span size='x-large' weight='bold'>MeaUpdater</span>");
        title_label.set_halign(gtk::Align::Center);
        title_label.set_margin_bottom(8);
        main_vbox.append(&title_label);

        // Version
        let version_label = Label::new(Some("Version 0.5"));
        version_label.set_markup("<span size='medium'>Version 0.5</span>");
        version_label.set_halign(gtk::Align::Center);
        version_label.set_margin_bottom(8);
        main_vbox.append(&version_label);

        // Developer info
        let developer_container = GtkBox::new(Orientation::Horizontal, 0);
        developer_container.set_halign(gtk::Align::Center);
        developer_container.set_margin_bottom(8);

        let year_label = Label::new(Some("2025 "));
        year_label.set_markup("<span size='medium'>2025 </span>");
        
        let link_button = Button::with_label("@mthakan");
        link_button.set_has_frame(false);
        link_button.add_css_class("link");
        
        // Link
        link_button.connect_clicked(|_| {
            if let Err(e) = open::that("https://github.com/mthakan") {
                eprintln!("The link could not be opened: {}", e);
            }
        });

        developer_container.append(&year_label);
        developer_container.append(&link_button);
        main_vbox.append(&developer_container);

        // Description
        let description_label = Label::new(Some("An update manager written in Rust for Debian-based systems"));
        description_label.set_markup("<span size='small' style='italic'>An update manager written in Rust for Debian-based systems</span>");
        description_label.set_halign(gtk::Align::Center);
        description_label.set_wrap(true);
        description_label.set_max_width_chars(50);
        description_label.set_margin_bottom(8);
        main_vbox.append(&description_label);

        // License
        let license_label = Label::new(Some("Licensed under the GNU General Public License v3.0 (GPLv3)"));
        license_label.set_markup("<span size='xx-small' alpha='70%'>Licensed under the GNU General Public License v3.0 (GPLv3)</span>");
        license_label.set_halign(gtk::Align::Center);
        license_label.set_wrap(true);
        license_label.set_max_width_chars(60);
        license_label.set_margin_bottom(17);
        main_vbox.append(&license_label);

        
        let tux_attribution = Label::new(Some("Tux by Larry Ewing, lewing@isc.tamu.edu"));
        tux_attribution.set_markup("<span size='xx-small' alpha='50%'>Tux by Larry Ewing, lewing@isc.tamu.edu</span>");
        tux_attribution.set_halign(gtk::Align::Center);
        tux_attribution.set_margin_bottom(15);
        main_vbox.append(&tux_attribution);

        
        let close_button = Button::with_label("Tamam");
        close_button.set_halign(gtk::Align::Center);
        close_button.set_size_request(100, 35);
        close_button.add_css_class("suggested-action");

        let window_clone = window.clone();
        close_button.connect_clicked(move |_| {
            window_clone.close();
        });

        main_vbox.append(&close_button);

        
        let css = "
        .link {
            color: #1976d2;
            text-decoration: underline;
        }
        .link:hover {
            color: #1565c0;
            background-color: rgba(25, 118, 210, 0.1);
        }
        ";

        let provider = gtk::CssProvider::new();
        provider.load_from_data(css);
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        window.set_child(Some(&main_vbox));
        window
    }
}
