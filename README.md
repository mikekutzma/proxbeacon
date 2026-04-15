# ProxBeacon

A rip-off of https://github.com/haylinmoore/board to use the PROXmobil3 as a
countdown clock. Mostly just whipped up quickly to play around with the
PROXmobil3, backend is mostly clanker slop, web UI is 100% certified clanker
slop. Deploy and docs are meat-typed.

## Build & Deploy

### Setup
This mostly assumes that you're building an deploying this from a mac, using
brew, etc. Though it probably wouldn't be too much work to tweak the justfile
for a linux box.

First create the usbstick that will allow for ssh, etc

``` bash
# Substitute the path to your external usb drive
just deploy-usb-autorun /Volumes/PROXMOBIL3
```

Then stick the usb in the dongle, plug in the POE, and hit the button on the
dongle to start the device. It will boot into the autorun script from the usb,
and you should be able to ssh into the device.

### Proxbeacon
At this point, you can build and deploy the source code for the app.

``` bash
# This will handle setup, build and deployment, optionally passing in the device
# hostname
just sync-proxbeacon-binary

# You can also run the recipes manually if you want to just rebuild
just setup-rust-build
just build-proxbeacon
just sync-proxbeacon-binary ilnx.lan
```

From there, you can ssh into the machine and run the app

``` bash
ssh root@ilnx.lan '/init/proxbeacon/exe'
```

You should see the logs in your ssh session that the server was started, you
should see the display on the device, and you should be able to open up the
admin ui at http://ilnx.lan:8123 (or whatever your hostname/ip of the device
is.)

To setup the app to run at boot, we'll copy over a systemd service file, and a
few init scripts to install the service (and stop other default service stuff).

``` bash
just deploy-proxbeacon-init
```

You can pull the usb stick out at this point and give this a reboot with

``` bash
ssh root@ilnx.lan reboot
```

At which point the device should boot directly into proxbeacon and the admin UI
should be available at the device host, same as before (http://ilnx.lan:8123)

### Cloudflare Tunnel

Something kind of fun is to add a cloudflare tunnel to the device so that you
can access the admin panel from outside of the local network. I like this
because I can pretty easily restrict access with access policies, and give folks
access easily as well. This really only works easily if your domain is also
hosted with cloudflare as far as I know.

The first step is to register the tunnel itself. That can happen on your local
dev machine and can be done using the cloudflared cli.

``` bash
just create-proxweb-tunnel
```

After that, you can deploy the tunnel binary and credentials over to the device.

``` bash
just deploy-proxweb-tunnel
```

You can test the tunnel out by starting it up manually

``` bash
ssh root@ilnx.lan '/init/cloudflared/exe tunnel --config /init/cloudflared/config.yml run'
```

and going to the url configured by your tunnel, in my case
https://proxweb.kutz.dev

The last step here, similarly to the proxbeacon app, is to add a systemd service
for the tunnel to start automatically on startup.

``` bash
just deploy-proxweb-tunnel-init
```

After reboot, the tunnel too should now start automatically.
