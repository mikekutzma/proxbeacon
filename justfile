ssh_key_path := env_var_or_default("SSH_PUBLIC_KEY_PATH", home_directory() / ".ssh/id_rsa.pub")
cloudflared_version := "2024.12.2"
cloudflared_arm := ".cache/cloudflared-linux-arm"
tunnel_name := "proxweb"
tunnel_hostname := "proxweb.kutz.dev"
default_ip := "ilnx.lan"

[doc("Builds the autorun for the usb that allows for inital ssh access. \
Really only needs to be run once. Probably mac specific.")]
deploy-usb-autorun usb_path:
    @test -f "{{ ssh_key_path }}" || (echo "ssh key not found: {{ ssh_key_path }}" && exit 1)
    @test -d "{{ usb_path }}" || (echo "usb not mounted: {{ usb_path }}" && exit 1)
    key=$(cat "{{ ssh_key_path }}") && \
        awk -v k="$key" '{gsub(/__SSH_PUBLIC_KEY__/, k); print}' usbstick/autorun.sh \
        > "{{ usb_path }}/autorun.sh"

[doc("Installs rust build prereqs. Depends on brew, mac specific.")]
setup-rust-build:
    brew install zig
    cargo install --locked cargo-zigbuild
    rustup target add armv7-unknown-linux-gnueabihf

[doc("Builds the app itself.")]
build-proxbeacon: setup-rust-build
    cd rust && cargo zigbuild --release --target armv7-unknown-linux-gnueabihf.2.28 -p proxbeacon --no-default-features --features framebuffer

[doc("Syncs the app binary to the device.")]
sync-proxbeacon-binary device_ip=default_ip: build-proxbeacon
    ssh root@{{ device_ip }} 'mount -o remount,rw / && mkdir -p /init/proxbeacon'
    cat rust/target/armv7-unknown-linux-gnueabihf/release/proxbeacon \
        | ssh root@{{ device_ip }} 'cat > /init/proxbeacon/exe && chmod +x /init/proxbeacon/exe && mount -o remount,ro /'

[doc("Deploys the proxbeacon systemd service file and autorun init script to the device.")]
deploy-proxbeacon-init device_ip=default_ip:
    ssh root@{{ device_ip }} 'mount -o remount,rw / && mkdir -p /init/proxbeacon /init/autorun'
    cat init/proxbeacon/proxbeacon.service \
        | ssh root@{{ device_ip }} 'cat > /init/proxbeacon/proxbeacon.service'
    cat init/autorun/99-proxbeacon.sh \
        | ssh root@{{ device_ip }} 'cat > /init/autorun/99-proxbeacon.sh && chmod +x /init/autorun/99-proxbeacon.sh && mount -o remount,ro /'
    cat init/autorun/98-setup.sh \
        | ssh root@{{ device_ip }} 'cat > /init/autorun/98-setup.sh && chmod +x /init/autorun/98-setup.sh && mount -o remount,ro /'

[doc("Installs cloudflare cli.")]
install-cloudflare-tooling:
    @which cloudflared >/dev/null || brew install cloudflared

[doc("Creates cloudflare tunnel. Happens on build local machine, only needs to \
be done once.")]
create-proxweb-tunnel: install-cloudflare-tooling
    @test -f "$HOME/.cloudflared/cert.pem" || cloudflared tunnel login
    cloudflared tunnel list | awk '$2=="{{ tunnel_name }}"{found=1} END{exit !found}' \
        || cloudflared tunnel create {{ tunnel_name }}
    cloudflared tunnel route dns {{ tunnel_name }} {{ tunnel_hostname }}

[doc("Pulls down the cloudflare tunnel binary copies it over to device. Also \
creates the credentials file and writes that to the device.")]
deploy-proxweb-tunnel device_ip=default_ip:
    @mkdir -p .cache
    @test -f "{{ cloudflared_arm }}" || curl -fL -o "{{ cloudflared_arm }}" \
        https://github.com/cloudflare/cloudflared/releases/download/{{ cloudflared_version }}/cloudflared-linux-arm
    tunnel_id=$(cloudflared tunnel list | awk '$2=="{{ tunnel_name }}"{print $1}') && \
        test -n "$tunnel_id" || (echo "run 'just setup-proxweb-tunnel' first" && exit 1); \
        creds="$HOME/.cloudflared/$tunnel_id.json"; \
        test -f "$creds" || (echo "creds not found: $creds" && exit 1); \
        ssh root@{{ device_ip }} 'mount -o remount,rw / && mkdir -p /init/cloudflared'; \
        cat "{{ cloudflared_arm }}" | ssh root@{{ device_ip }} \
            'cat > /init/cloudflared/exe && chmod +x /init/cloudflared/exe'; \
        cat "$creds" | ssh root@{{ device_ip }} 'cat > /init/cloudflared/creds.json'; \
        awk -v id="$tunnel_id" -v host="{{ tunnel_hostname }}" \
            '{gsub(/__TUNNEL_ID__/, id); gsub(/__TUNNEL_HOST__/, host); print}' \
            init/cloudflared/config.yaml \
            | ssh root@{{ device_ip }} 'cat > /init/cloudflared/config.yml && mount -o remount,ro /'

[doc("Deploys the proxweb tunnel systemd service file and autorun init script to the device.")]
deploy-proxweb-tunnel-init device_ip=default_ip:
    ssh root@{{ device_ip }} 'mount -o remount,rw / && mkdir -p /init/cloudflared /init/autorun'
    cat init/cloudflared/cloudflared.service \
        | ssh root@{{ device_ip }} 'cat > /init/cloudflared/cloudflared.service'
    cat init/autorun/99-cloudflared.sh \
        | ssh root@{{ device_ip }} 'cat > /init/autorun/99-cloudflared.sh && chmod +x /init/autorun/99-cloudflared.sh && mount -o remount,ro /'

run-proxbeacon device_ip=default_ip:
    ssh root@{{ device_ip }} '/init/proxbeacon/exe & /init/cloudflared/exe tunnel --config /init/cloudflared/config.yml run'
