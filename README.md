# rust-classicube-relay

Library for sending and receiving relay messages from the [MCGalaxy-Relay-Plugin](https://github.com/SpiralP/MCGalaxy-Relay-Plugin).

## Examples

Receive data from other players:

```rust no_run
use classicube_relay::RelayListener;

let channel = 200;
let mut listener = RelayListener::new(channel).unwrap();
listener.on(move |player_id: u8, data: &[u8]| {
    println!("Player {} sent {:?}", player_id, data);
});
```

Send data to another player by id:

```rust no_run
use classicube_relay::{packet::PlayerScope, Stream};

let channel = 200;
let mut stream = Stream::new(
    b"hello!".to_vec(),
    PlayerScope::new(123)
).unwrap();
for packet in stream.packets().unwrap() {
    let mut data = packet.encode().unwrap();
    unsafe {
        classicube_sys::CPE_SendPluginMessage(channel, data.as_mut_ptr());
    }
}
```

Send data to all players in my same map:

```rust no_run
use classicube_relay::{packet::MapScope, Stream};

let channel = 200;
let mut stream = Stream::new(
    b"hello!".to_vec(),
    MapScope::default()
).unwrap();
for packet in stream.packets().unwrap() {
    let mut data = packet.encode().unwrap();
    unsafe {
        classicube_sys::CPE_SendPluginMessage(channel, data.as_mut_ptr());
    }
}
```
