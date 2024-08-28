# bvr_chirp

<div style="text-align: center;">
  <img alt="BVR Chirp Logo" src="logo.png" width="380" />
</div>


BVR Chirp is a bridge between Blue Iris (MQTT) to send a messaging service (Discord/matrix).

### THIS PROJECT IS IN ALPHA. It should work, but don't expect too much yet.

# Installation

### Installing from source

Ensure you have Rust installed: https://doc.rust-lang.org/book/ch01-01-installation.html

```
git clone https://codeberg.org/CeeBee/bvr_chirp
cd bvr_chirp
cargo build
```

### Running from release

Download the latest version from the [releases](https://codeberg.org/CeeBee/bvr_chirp/releases) page.

Extract archive to a location you want to run it from.

# Configuration

You need to be running a separate MQTT broker. The one I personally use and tested with is [rumqttd](https://github.com/bytebeamio/rumqtt/tree/main/rumqttd).

I found it easiest to use by running it with docker:

```
docker run -p 1883:1883 -p 1884:1884 -v /absolute/path/to/rumqttd.toml:/rumqttd.toml -it bytebeamio/rumqttd -c /rumqttd.toml
```

You need to configure a username and password for the `rumqttd.toml` file. It is also important that the `max_payload_size` in the `rumqttd.toml` file is set the same as the MQTT client max_packet_size (see below). You can see an example in the file `rumqttd_example.toml`

Once you have a broker setup, you need to configure bvr_chirp. The sample config looks like such 

```toml
[mqtt_config]
host="127.0.0.1"
port=1884
max_packet_size=2048000
topic="BlueIris/alert"
device_id="BVR Chirp Bot"
username="YOUR_USERNAME"
password="YOUR_PASSWORD"

[messaging_config]
service_type="discord"
token="YOUR_TOKEN_HERE"
name="BVR Chirp Bot"
host="https://matrix.org"
username="YOUR_USERNAME"
password="YOUR_PASSWORD"
endpoint="http://<BLUE_IRIS_IP>:81"
```

Some notes:

* max_packet_size: must be set to a value higher than the largest image will be transmitted, otherwise MQTT will refuse the message for being too large
* topic: this can be anything you want, but you must make sure your sender (Blue Iris) and bvr_chirp are using the same topic
* service_type: for now this is only Discord, with matrix half implemented. Eventually I would like to have other services like Telegram, Signal, Whatsapp, or whatever users need/want
* token: this the auth token for the messaging service. For example, this would be your Discord bot API token
* host (under messaging_config): this is mainly needed for matrix to specify the homeserver for the bot  
* endpoint: this is your Blue Iris URL

The links to Blue Iris in messages look like this:

`http://192.168.1.10:81/login.htm?page=%2Fui3.htm%3Frec=@195238907624039%26cam=FrontDoor%26m=1`

This will open the Blue Iris web UI directly to the alert in question, and will open it in fullscreen.



# Running

Once built run it with:

`cargo run bvr_chirp.cfg`

If downloaded, run it with:

`./bvr_chirp bvr_chirp.cfg`

# TODO:
- [x] Get this code published
- [ ] Configurable version for the MQTT client (v3 or v5), right now it's hardcoded for v5 
- [ ] Add more messaging services
- [ ] Web interface for configuration (don't hold your breath)

# The Name

"BVR" is a recursive name for BVR Video Recorder. "Chirp" is a sound a beaver can make.

BVR will eventually be a project suite, but that's all I'll say for now.