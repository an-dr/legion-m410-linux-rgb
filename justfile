# Recipes for building, testing and installing. `just` lists them.

bin := env_var('HOME') / '.local/bin'
apps := env_var('HOME') / '.local/share/applications'
# Where the first version of this app kept itself, with an extension folder beside the binary.
old_data := env_var('HOME') / '.local/share/legion-m410-linux-rgb'
exe := 'legion-m410-linux-rgb'

default:
    @just --list

# Run from the source tree
run:
    cargo run -p legion-m410-linux-rgb-app

# Unit tests, which need no mouse
test:
    cargo test

# Build the optimized app
build:
    cargo build --release -p legion-m410-linux-rgb-app

# Install for the current user: the app in ~/.local/bin and a launcher entry
install: build
    rm -rf {{old_data}}
    rm -f {{bin}}/{{exe}}
    install -Dm755 target/release/{{exe}} {{bin}}/{{exe}}
    install -Dm644 res/{{exe}}.desktop {{apps}}/{{exe}}.desktop
    @echo "Installed. Run '{{exe}}' or find 'Legion M410 RGB' in your launcher."

uninstall:
    rm -rf {{old_data}}
    rm -f {{bin}}/{{exe}} {{apps}}/{{exe}}.desktop

# Let your user open the mouse and its receiver (needs sudo, once per machine)
install-udev:
    sudo install -Dm644 res/61-legion-m410.rules /etc/udev/rules.d/61-legion-m410.rules
    sudo udevadm control --reload
    sudo udevadm trigger --subsystem-match=hidraw
