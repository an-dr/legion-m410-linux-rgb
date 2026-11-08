# Recipes for building, testing and installing. `just` lists them.

data := env_var('HOME') / '.local/share/legion-m410-linux-rgb'
bin := env_var('HOME') / '.local/bin'
apps := env_var('HOME') / '.local/share/applications'
exe := 'legion-m410-linux-rgb'

default:
    @just --list

# Run from the source tree, rebuilding and hot-reloading the extension on every save
dev:
    bones-develop dev

# Unit tests, plus the checks that run the extension against a fake mouse
test:
    cargo test

# Build the optimized app and its extension
build:
    cargo build --release -p legion-m410-linux-rgb-app
    bones-develop build

# Install for the current user: the app and its extension in ~/.local/share, a launcher in ~/.local/bin
install: build
    install -Dm755 target/release/{{exe}} {{data}}/{{exe}}
    install -Dm644 .bones/extensions/legion_m410_linux_rgb.wasm {{data}}/extensions/legion_m410_linux_rgb.wasm
    ln -sf {{data}}/{{exe}} {{bin}}/{{exe}}
    install -Dm644 res/{{exe}}.desktop {{apps}}/{{exe}}.desktop
    @echo "Installed. Run '{{exe}}' or find 'Legion M410 RGB' in your launcher."

uninstall:
    rm -rf {{data}}
    rm -f {{bin}}/{{exe}} {{apps}}/{{exe}}.desktop

# Let your user open the mouse and its receiver (needs sudo, once per machine)
install-udev:
    sudo install -Dm644 res/61-legion-m410.rules /etc/udev/rules.d/61-legion-m410.rules
    sudo udevadm control --reload
    sudo udevadm trigger --subsystem-match=hidraw
