# meaupdater
A simple update manager written in Rust with GTK4 for Debian-based systems.

---

![screenshot](https://github.com/user-attachments/assets/8ba432a0-da05-430b-9f72-738b967ca8e1)

---

## Features

- Check for updates and group them by type: software, security, kernel
- Download and install updates
- Manage APT repositories with the built-in Repository Manager
- Kernel Manager: view, install, remove, and set default kernels
- Driver Manager: detect, install, and manage hardware drivers
- Clean and user-friendly GTK4 interface
- Built with Rust for performance and reliability

---

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3).  
See the [LICENSE](https://www.gnu.org/licenses/gpl-3.0.en.html) file for details.
> If you use this project or its code, **please attribute the original author**.
---

## Installation 
```sh
sudo apt update && sudo apt install -y build-essential pkg-config curl git libgtk-3-dev libglib2.0-dev libpango1.0-dev libgdk-pixbuf2.0-dev libatk1.0-dev libadwaita-1-dev libgraphene-1.0-dev
git clone https://github.com/mthakan/meaupdater.git
cd meaupdater
cargo build --release
